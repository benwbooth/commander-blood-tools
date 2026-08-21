#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define PRESENTATION_ACTIVE_FLAG 0x01u
#define PRESENTATION_REVERSE_FLAG 0x01u
#define PRESENTATION_UI_REDRAW_FLAG 0x04u
#define PRESENTATION_BOX_OPEN_STEPS BLOODPRG_SELECTED_MASK_COUNT
#define PRESENTATION_TRANSITION_END 10
#define PRESENTATION_CLOSE_BASE 100
#define PRESENTATION_CLOSE_START 106
#define PRESENTATION_BOX_FILL_COLOR 0xE0u
#define PRESENTATION_BOX_FRAME_COLOR 0xEFu
#define PRESENTATION_CONTENT_TOP 10u
#define PRESENTATION_CONTENT_HEIGHT 140u
#define PRESENTATION_FRAME_HEIGHT 130u
#define PRESENTATION_SCREEN_WIDTH 320u
#define PRESENTATION_SCREEN_HEIGHT 200u
#define PRESENTATION_NOISE_MODE 3u
#define PRESENTATION_LINE_ID 2u
#define PRESENTATION_LINE_RECORD_BYTES 16u
#define PRESENTATION_TEXT_TABLE_STRIDE 128

#define PRESENTATION_REMAP(table, x, y, width, height) \
    framebuffer_rect_palette_remap( \
        (const cb_u8 CB_FAR *)(table), (x), (y), (width), (height))

void CB_NEAR screen_mode_update(cb_u16 queued_scene_link_target)
{
    const volatile bloodprg_rect_i16 CB_NEAR *box;
    volatile char CB_NEAR *selected_record;
    volatile char CB_NEAR *source;
    volatile char CB_GAME_DATA *destination;
    volatile cb_u8 CB_FAR *saved_display;
    cb_u16 scene_link_target;
    cb_i16 phase;
    cb_i16 table_index;
    cb_i8 first_character;
    cb_u8 character;

    if ((presentation_mode_flag_27e1 & PRESENTATION_ACTIVE_FLAG) == 0u) {
        return;
    }

    scene_link_target = queued_scene_link_target;
    if ((vm_c2_presentation_gate & 1u) != 0u) {
        goto dispatch_scene;
    }

    phase = presentation_box_phase;
    if (phase == 0) {
        presentation_text_origin_y = 1u;
        selected_mask_index = 0;
        presentation_mode_previous_state = 15u;
        presentation_box_phase = 1;
        entity_flag_state_transition(31u);
        snd_play_clip(1);
        return;
    }

    if (phase >= PRESENTATION_CLOSE_BASE) {
        table_index = (cb_i16)(phase - PRESENTATION_CLOSE_BASE);
        if (table_index == 0) {
            resource_vertical_offset = 0u;
            presentation_mode_flag_27e1 = 0u;
            vm_ui_flags &= (cb_u8)~PRESENTATION_UI_REDRAW_FLAG;
            presentation_box_phase = 0;
            presentation_completion_audio_pending = 1u;
            presentation_text_origin_y = 8u;
            nav_screen_rebuild_pending = 1u;
            if ((presentation_mode_flag_27e0 & PRESENTATION_REVERSE_FLAG) != 0u) {
                resource_variant = 12u;
                presentation_mode_flag_27e0 = 0u;
            } else {
                ship_3d_hud_palette_snapshot_and_camera_reset();
            }
            return;
        }

        --presentation_box_phase;
        box = &presentation_box_animation_rects[table_index - 1];
        framebuffer_rect_fill(
                PRESENTATION_BOX_FILL_COLOR,
                (cb_u16)box->x,
                (cb_u16)box->y,
                (cb_u16)box->width,
                (cb_u16)box->height);
        composite_draw_a(
                PRESENTATION_BOX_FRAME_COLOR,
                (cb_u16)box->x,
                (cb_u16)box->y,
                (cb_u16)box->width,
                (cb_u16)box->height);
        return;
    }

    table_index = (cb_i16)(phase - 1);
    if (table_index < PRESENTATION_BOX_OPEN_STEPS) {
        ++presentation_box_phase;
        box = &presentation_box_animation_rects[table_index];
        framebuffer_rect_fill(
                PRESENTATION_BOX_FILL_COLOR,
                (cb_u16)box->x,
                (cb_u16)box->y,
                (cb_u16)box->width,
                (cb_u16)box->height);
        composite_draw_a(
                PRESENTATION_BOX_FRAME_COLOR,
                (cb_u16)box->x,
                (cb_u16)box->y,
                (cb_u16)box->width,
                (cb_u16)box->height);
        return;
    }

    table_index = (cb_i16)(table_index - PRESENTATION_BOX_OPEN_STEPS);
    if (table_index < PRESENTATION_TRANSITION_END - 7) {
        ++presentation_box_phase;
        PRESENTATION_REMAP(
                bloodprg_sprite_remap_6011,
                0u,
                0u,
                PRESENTATION_SCREEN_WIDTH,
                PRESENTATION_SCREEN_HEIGHT);
        framebuffer_noise_rect(
                PRESENTATION_NOISE_MODE,
                1u,
                PRESENTATION_CONTENT_TOP,
                PRESENTATION_SCREEN_WIDTH - 1u,
                PRESENTATION_FRAME_HEIGHT);
        return;
    }

    PRESENTATION_REMAP(
            bloodprg_sprite_remap_6011,
            0u,
            0u,
            PRESENTATION_SCREEN_WIDTH,
            PRESENTATION_SCREEN_HEIGHT);
    saved_display = bloodprg_display_buffer;
    bloodprg_display_buffer = bloodprg_secondary_buffer;
    bloodprg_secondary_buffer = saved_display;
    PRESENTATION_REMAP(
            bloodprg_sprite_remap_6011,
            0u,
            0u,
            PRESENTATION_SCREEN_WIDTH,
            PRESENTATION_SCREEN_HEIGHT);
    framebuffer_rect_fill(
            0u,
            0u,
            0u,
            PRESENTATION_SCREEN_WIDTH,
            PRESENTATION_CONTENT_HEIGHT);
    saved_display = bloodprg_display_buffer;
    bloodprg_display_buffer = bloodprg_secondary_buffer;
    bloodprg_secondary_buffer = saved_display;
    scene_link_target = PRESENTATION_CONTENT_HEIGHT;

    selected_record = vm_record_string_slots[(cb_i16)selected_mask_index];
    if (*selected_record == 0) {
        framebuffer_noise_rect(
                PRESENTATION_NOISE_MODE,
                1u,
                PRESENTATION_CONTENT_TOP,
                PRESENTATION_SCREEN_WIDTH - 1u,
                PRESENTATION_FRAME_HEIGHT);
        selected_mask_overlay();
        if ((mouse_primary_pressed & 1u) != 0u) {
            goto accept_input;
        }
        return;
    }

    (void)vm_c2_descript_lookup(
            (const volatile cb_u8 CB_FAR *)selected_record);
    if ((snd_music_voc_name_changed & 1u) != 0u) {
        snd_driver_call();
        snd_stream_source_load(presentation_music_voc_path);
    }
    snd_stream_start();

    first_character = (cb_i8)vm_record_string_slots[0][0];
    descript_text_record_cursor =
            (volatile char CB_NEAR *)descript_text_record_table
            + (cb_i16)first_character * PRESENTATION_TEXT_TABLE_STRIDE;
    descript_text_records_remaining = descript_text_record_count;
    resource_vertical_offset = PRESENTATION_CONTENT_TOP;
    list_d8c_sequence_index = 0u;
    goto pump_record;

dispatch_scene:
    if ((mouse_primary_pressed & 1u) != 0u) {
        goto accept_input;
    }
    dlg_line_id_scene_dispatch(scene_link_target);
    if ((vm_c2_presentation_gate & 1u) != 0u) {
        if ((resource_frame_presented & 1u) != 0u) {
            list_walk_f18();
            selected_mask_overlay();
        }
        return;
    }

pump_record:
    if (descript_text_records_remaining == 0u) {
        descript_centered_text_cursor =
                (volatile char CB_NEAR *)&descript_centered_text_events[0];
        nav_screen_rebuild_pending = 1u;
        presentation_box_phase = 7;
        return;
    }

    --descript_text_records_remaining;
    source = descript_text_record_cursor;
    descript_text_record_cursor += PRESENTATION_LINE_RECORD_BYTES;
    destination = vm_scene_name_buffer;
    do {
        character = (cb_u8)*source++;
        *destination++ = (char)character;
    } while (character != 0u);
    vm_active_line = PRESENTATION_LINE_ID;
    goto dispatch_scene;

accept_input:
    if ((presentation_mode_flag_27e0 & PRESENTATION_REVERSE_FLAG) != 0u) {
        presentation_box_phase = PRESENTATION_CLOSE_START;
        if ((vm_c2_presentation_gate & 1u) != 0u) {
            presentation_update_1fb2();
        }
    } else {
        manu3_animation_selector_current = 0u;
        manu3_animation_selector_request = 14u;
        snd_driver_call();
        snd_play_clip(1);
        presentation_update_1fb2();
        ++selected_mask_index;
        if (selected_mask_index == PRESENTATION_BOX_OPEN_STEPS) {
            selected_mask_index = 0;
        }
        presentation_box_phase = 7;
    }

    saved_display = bloodprg_display_buffer;
    bloodprg_display_buffer = bloodprg_secondary_buffer;
    framebuffer_rect_fill(
            0u,
            0u,
            PRESENTATION_CONTENT_TOP,
            PRESENTATION_SCREEN_WIDTH,
            PRESENTATION_FRAME_HEIGHT);
    bloodprg_display_buffer = saved_display;
}
