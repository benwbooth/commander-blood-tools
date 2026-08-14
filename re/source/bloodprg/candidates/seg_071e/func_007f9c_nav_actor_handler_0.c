#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_0_UI_GATE 0x10u

void CB_NEAR nav_actor_handler_0(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    cb_u8 flags;
    cb_u8 deferred_gate;
    cb_u8 second_pass_prepared;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_0_UI_GATE) == 0u
            || nav_actor_0_busy != 0u) {
        return;
    }

    flags = line->flags;
    second_pass_prepared =
            (cb_u8)((flags & NAV_ACTOR_LOADED_FLAG) != 0u);

    if ((flags & NAV_ACTOR_PRESENT_FLAG) != 0u) {
        if ((flags & NAV_ACTOR_READY_FLAG) != 0u) {
            nav_target_presentation_state = 0u;
            nav_actor_presentation_state = 10u;
            entity_flag_state_transition(0u);
            entity_flag_state_transition(4u);
            (void)presentation_line_helper(line);
            second_pass_prepared = 1u;

            if (line->frame_index == 1u) {
                nav_camera_view_state = 8u;
            } else if (nav_camera_view_state == 0u) {
                line->flags = 7u;
                nav_deferred_record_type = 0x00c1u;
                nav_transition_pending = 1u;
                entity_flag_state_transition(4u);
                nav_location_panel_active = 0u;
                vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
                return;
            }
        }

        if ((nav_location_panel_active & 1u) != 0u) {
            return;
        }
        line->resource_id = 0x14u;
        nav_presentation_reverse = 1u;
        line->flags = 0u;
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    }

    if (nav_deferred_record_link == 0u) {
        return;
    }
    deferred_gate = nav_presentation_reverse;
    if ((deferred_gate |= nav_location_panel_active) == 0u) {
        return;
    }

    if (second_pass_prepared == 0u) {
        entity_flag_state_transition(4u);
        line->resource_id = 0x14u;
        snd_play_clip(5);
    }

    if (!presentation_line_helper(line)) {
        return;
    }
    if ((nav_location_panel_active & 1u) == 0u) {
        nav_deferred_record_link = 0u;
        entity_flag_state_transition(4u);
        line->flags = 0u;
    } else {
        line->flags = NAV_ACTOR_PRESENT_FLAG;
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
        line->resource_id = 0x12u;
    }
}
