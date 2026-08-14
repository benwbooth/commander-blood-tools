#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_vm.h"

#define NAV_CHOICE_NONE 0xffffu
#define NAV_CHOICE_RECORD_NAME_OFFSET 4u
#define NAV_CHOICE_UI_ACTIVE 0x04u

void CB_NEAR nav_choice_handler_2(void)
{
    const volatile cb_u16 *contact;
    volatile cb_u16 *target;
    cb_i16 selection;
    cb_u16 contact_record;
    cb_u16 selected_target;
    int transition_complete;

    target = nav_kind2_target_offsets;
    if ((nav_choice_phase & 1u) != 0u) {
        contact = nav_contact_slot_words;
        for (;;) {
            contact_record = *contact++;
            if (contact_record == 0u) {
                continue;
            }
            if (contact_record == NAV_CHOICE_NONE) {
                *target = NAV_CHOICE_NONE;
                break;
            }
            *target++ =
                    (cb_u16)(contact_record + NAV_CHOICE_RECORD_NAME_OFFSET);
        }

        target = nav_kind2_target_offsets;
        framebuffer_transition_current_step = 0u;
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified((const cb_u16 *)target);
        presentation_list_editing = 0u;
        ++nav_choice_phase;
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

    target = nav_kind2_target_offsets;
    selection = list_widget_layout_unified((const cb_u16 *)target);
    if (selection == (cb_i16)NAV_CHOICE_NONE) {
        return;
    }

    selected_target = target[(cb_u16)selection];
    if (selected_target != NAV_CHOICE_NONE) {
        selected_target =
                (cb_u16)(selected_target - NAV_CHOICE_RECORD_NAME_OFFSET);
        nav_deferred_record_link = selected_target;
        render_update_flag_2751 = 1u;
    }

    nav_console_selected_item = 0u;
    vm_ui_flags &= (cb_u8)~NAV_CHOICE_UI_ACTIVE;
}
