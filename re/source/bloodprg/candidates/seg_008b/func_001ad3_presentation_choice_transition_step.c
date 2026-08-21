#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_vm.h"

#define PRESENTATION_UI_ACTIVE 0x04u
#define PRESENTATION_CHOICE_NONE 0xffffu
#define PRESENTATION_CHOICE_SPECIAL_INDEX 4
#define PRESENTATION_CHOICE_SPECIAL_RESULT 7u

void CB_NEAR presentation_choice_transition_step(void)
{
    cb_i16 selection;
    cb_u16 result;
    int transition_complete;

    if ((presentation_choice_active & 1u) == 0u) {
        return;
    }

    vm_ui_flags |= PRESENTATION_UI_ACTIVE;
    if ((presentation_choice_phase & 1u) != 0u) {
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified(
                presentation_choice_items, presentation_choice_items);
        presentation_list_editing = 0u;
        framebuffer_transition_current_step = 0u;
        framebuffer_transition_total_steps = 6u;
        ++presentation_choice_phase;
    }

    if ((presentation_choice_phase & 2u) != 0u) {
        transition_complete =
                framebuffer_transition_total_steps
                == framebuffer_transition_current_step;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_target_rect);
        if (!transition_complete) {
            return;
        }
        presentation_choice_phase = 0u;
    }

    selection = list_widget_layout_unified(
            presentation_choice_items, presentation_choice_items);
    if (selection < 0) {
        return;
    }

    if (presentation_choice_items[(cb_u16)selection]
            != PRESENTATION_CHOICE_NONE) {
        if (selection == PRESENTATION_CHOICE_SPECIAL_INDEX) {
            result = PRESENTATION_CHOICE_SPECIAL_RESULT;
        } else {
            result = (cb_u16)selection + 1u;
        }
        presentation_choice_result = result;
    }

    vm_ui_flags &= (cb_u8)~PRESENTATION_UI_ACTIVE;
    presentation_choice_active = 0u;
}
