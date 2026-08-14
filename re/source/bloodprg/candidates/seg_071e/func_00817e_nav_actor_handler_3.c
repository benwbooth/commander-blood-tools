#include "../include/bloodprg_nav.h"

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_3_UI_GATE 0x40u

void CB_NEAR nav_actor_handler_3(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    if ((vm_ui_flags & NAV_ACTOR_HANDLER_3_UI_GATE) == 0u) {
        return;
    }

    line->flags |= NAV_ACTOR_PRESENT_FLAG;
    if ((line->flags & NAV_ACTOR_READY_FLAG) != 0u) {
        nav_actor_presentation_state = 13u;
        if ((presentation_mode_flag_27e1 & 1u) != 0u
                && nav_actor_zoom_counter < 100) {
            nav_actor_zoom_counter = 106;
            if ((vm_c2_presentation_gate & 1u) != 0u) {
                presentation_update_1fb2();
            }
        }

        mouse_primary_pressed = 0u;
        mouse_press_pending = 0u;
        if (presentation_line_helper(line)) {
            entity_flag_state_transition(4u);
            line->flags = NAV_ACTOR_PRESENT_FLAG;
            if ((presentation_mode_flag_27e1 & 1u) == 0u) {
                presentation_mode_flag_27e1 = 1u;
                vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
            }
        }
    }

    if ((presentation_mode_flag_27e1 & 1u) != 0u
            && (line->flags & NAV_ACTOR_LOADED_FLAG) != 0u) {
        nav_actor_completion_latch = 1u;
    }
}
