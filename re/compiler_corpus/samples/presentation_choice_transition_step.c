/* Codegen probe for BLOODPRG 0x001AD3. */

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;

#define FAR far
#define NEAR near

typedef struct rect_i16_probe {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16_probe;

extern volatile u8 presentation_choice_active_probe;
extern volatile u8 presentation_choice_phase_probe;
extern const u16 presentation_choice_items_probe[];
extern const i16 presentation_choice_target_rect_probe[4];
extern volatile u16 presentation_choice_result_probe;
extern volatile u8 presentation_list_editing_probe;
extern volatile u8 vm_ui_flags_probe;
extern volatile u8 transition_current_step_probe;
extern volatile u8 transition_total_steps_probe;
extern volatile u16 save_slot_row_x_probe;

i16 FAR list_widget_layout_unified_probe(const u16 NEAR *items);
void FAR framebuffer_rect_interpolate_and_remap_step_probe(
        const rect_i16_probe NEAR *source,
        const rect_i16_probe NEAR *target);

#pragma aux list_widget_layout_unified_probe parm [si] value [ax]
#pragma aux framebuffer_rect_interpolate_and_remap_step_probe \
        parm [si] [di] modify exact []

void NEAR presentation_choice_transition_step_probe(void)
{
    i16 selection;
    u16 result;
    int transition_complete;

    if ((presentation_choice_active_probe & 1u) == 0u) {
        return;
    }

    vm_ui_flags_probe |= 4u;
    if ((presentation_choice_phase_probe & 1u) != 0u) {
        presentation_list_editing_probe = 1u;
        (void)list_widget_layout_unified_probe(presentation_choice_items_probe);
        presentation_list_editing_probe = 0u;
        transition_current_step_probe = 0u;
        transition_total_steps_probe = 6u;
        ++presentation_choice_phase_probe;
    }

    if ((presentation_choice_phase_probe & 2u) != 0u) {
        transition_complete =
                transition_total_steps_probe == transition_current_step_probe;
        framebuffer_rect_interpolate_and_remap_step_probe(
                (const rect_i16_probe NEAR *)&save_slot_row_x_probe,
                (const rect_i16_probe NEAR *)
                    presentation_choice_target_rect_probe);
        if (!transition_complete) {
            return;
        }
        presentation_choice_phase_probe = 0u;
    }

    selection = list_widget_layout_unified_probe(presentation_choice_items_probe);
    if (selection < 0) {
        return;
    }

    if (presentation_choice_items_probe[(u16)selection] != 0xffffu) {
        result = selection == 4 ? 7u : (u16)selection + 1u;
        presentation_choice_result_probe = result;
    }

    vm_ui_flags_probe &= (u8)~4u;
    presentation_choice_active_probe = 0u;
}
