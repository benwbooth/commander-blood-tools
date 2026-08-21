#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define NAV_CHOICE_UI_ACTIVE 0x04u
#define OPTION_MENU_TEXT 0u
#define OPTION_MENU_MUSIC 1u
#define OPTION_MENU_SAVE 2u
#define OPTION_MENU_LOAD 3u
#define OPTION_MENU_QUIT 4u

void CB_NEAR nav_choice_handler_4(void)
{
    const cb_u16 *items;
    cb_i16 selection;
    cb_u8 choice;
    int transition_complete;

    items = (const cb_u16 *)option_menu_label_pointers;
    if ((nav_choice_phase & 1u) != 0u) {
        framebuffer_transition_current_step = 0u;
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified(
                items, option_menu_label_pointers);
        presentation_list_editing = 0u;
        ++nav_choice_phase;

        *(volatile bloodprg_rect_i16 *)presentation_choice_target_rect =
                *(const volatile bloodprg_rect_i16 *)
                    presentation_choice_current_rect;
        items = (const cb_u16 *)(presentation_choice_current_rect + 4);
    }

    if ((nav_choice_phase & 2u) != 0u) {
        transition_complete = framebuffer_transition_total_steps
                == framebuffer_transition_current_step;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    nav_choice_animation_target_rect);
        if (!transition_complete) {
            return;
        }
        nav_choice_phase = 0u;
    }

    selection = list_widget_layout_unified(
            items, option_menu_label_pointers);
    if (selection < 0) {
        return;
    }

    choice = (cb_u8)selection;
    if (choice == OPTION_MENU_TEXT || choice > 0x80u) {
        presentation_choice_active = 1u;
        presentation_choice_phase = 1u;
    } else if (choice == OPTION_MENU_MUSIC) {
        if ((voc_playback_enabled & 1u) != 0u) {
            if ((voc_tablo2_active & 1u) != 0u) {
                snd_driver_pending_flag = 0u;
                voc_tablo2_active = 0u;
                option_menu_label_pointers[1] = option_menu_music_on_label;
            } else {
                voc_tablo2_reset_gate = 0u;
                snd_driver_pending_flag = 0u;
                voc_tablo2_active = 1u;
                option_menu_label_pointers[1] = option_menu_music_off_label;
                if ((voc_playback_enabled & 1u) != 0u) {
                    snd_stream_source_load(voc_tablo2_path);
                    snd_stream_start();
                }
            }
        }
    } else if (choice == OPTION_MENU_SAVE) {
        nav_choice_motion_active = 1u;
        nav_choice_left_motion_active = 1u;
    } else if (choice == OPTION_MENU_LOAD) {
        nav_choice_motion_active = 1u;
        nav_choice_right_motion_active = 1u;
    } else if (choice == OPTION_MENU_QUIT) {
        ship_3d_nav_choice_sound_gate = 2u;
        mouse_primary_pressed = 0u;
        mouse_press_pending = 0u;
    }

    nav_console_selected_item = 0u;
    vm_ui_flags &= (cb_u8)~NAV_CHOICE_UI_ACTIVE;
}
