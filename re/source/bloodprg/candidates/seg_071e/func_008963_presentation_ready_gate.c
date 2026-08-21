#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define PRESENTATION_WORD_CHOICE_UI_FLAG 0x04u
#define PRESENTATION_WORD_CHOICE_PHASE_MASK 0x07u
#define PRESENTATION_WORD_CHOICE_CENTER_X 225u
#define PRESENTATION_WORD_CHOICE_STEPS 4u

void CB_FAR presentation_ready_gate(void)
{
    const volatile cb_u16 CB_NEAR *items;
    cb_i16 selection;
    int transition_complete;

    if ((vm_presentation_active & 1u) == 0u
            || (vm_word_choice_active & 1u) == 0u
            || (vm_presentation_request_flags & 2u) != 0u) {
        return;
    }

    items = vm_presentation_word_buffer;
    if (*items == 0u) {
        return;
    }

    if ((vm_presentation_word_choice_phase
            & PRESENTATION_WORD_CHOICE_PHASE_MASK) == 0u) {
        vm_ui_flags |= PRESENTATION_WORD_CHOICE_UI_FLAG;
        ++vm_presentation_word_choice_phase;
        ship_3d_target_layout_preserve_widths = 0u;
        ship_3d_target_layout_center_x = PRESENTATION_WORD_CHOICE_CENTER_X;
        ship_3d_target_layout_extra_entry = 0u;
        framebuffer_transition_current_step = 0u;
        framebuffer_transition_total_steps = PRESENTATION_WORD_CHOICE_STEPS;
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified(
                (const cb_u16 CB_NEAR *)items, vm_dic_words);
        presentation_list_editing = 0u;
        presentation_word_choice_target_rect.x =
                presentation_choice_current_rect[0];
        presentation_word_choice_target_rect.width =
                presentation_choice_current_rect[2];
    }

    if ((vm_presentation_word_choice_phase & 2u) == 0u) {
        transition_complete = framebuffer_transition_current_step
                == framebuffer_transition_total_steps;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    &presentation_word_choice_target_rect);
        if (!transition_complete) {
            return;
        }
        ++vm_presentation_word_choice_phase;
    }

    if ((vm_presentation_word_choice_phase & 1u) == 0u) {
        selection = list_widget_layout_unified(
                (const cb_u16 CB_NEAR *)items, vm_dic_words);
        if (selection < 0) {
            return;
        }
        vm_presentation_selected_word = items[(cb_u16)selection];
        framebuffer_transition_current_step = 0u;
        ++vm_presentation_word_choice_phase;
        return;
    }

    transition_complete = framebuffer_transition_current_step
            == framebuffer_transition_total_steps;
    framebuffer_rect_interpolate_and_remap_step(
            (const bloodprg_rect_i16 CB_NEAR *)
                &presentation_word_choice_target_rect,
            (const bloodprg_rect_i16 CB_NEAR *)
                presentation_choice_current_rect);
    if (!transition_complete) {
        return;
    }

    vm_block_match_value = vm_presentation_selected_word;
    vm_word_choice_active = 0u;
    vm_presentation_defer_a = 0u;
    vm_text_display_active = 0u;
    vm_dialogue_hold_complete = 0u;
    vm_presentation_word_choice_phase = 0u;
    vm_presentation_word_buffer[0] = 0u;
    vm_presentation_request_flags &= (cb_u8)~1u;
}
