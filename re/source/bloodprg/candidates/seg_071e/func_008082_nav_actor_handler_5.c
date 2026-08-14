#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_TRANSITION_FLAG 0x02u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_5_UI_GATE 0x10u
#define NAV_CAMERA_VIEW_ACTIVE_FLAG 0x01u

void CB_NEAR nav_actor_handler_5(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    cb_u8 transition_flags;
    int line_completed;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_5_UI_GATE) == 0u) {
        return;
    }

    if ((nav_actor_5_active & 1u) == 0u) {
        line->flags |= NAV_ACTOR_PRESENT_FLAG;
        transition_flags = line->flags;
        if ((transition_flags & NAV_ACTOR_READY_FLAG) == 0u) {
            goto transition_test;
        }
    }

    if ((nav_actor_1_busy | nav_actor_0_busy) != 0u) {
        nav_actor_5_active = 1u;
        nav_location_panel_active = 0u;
        return;
    }

    entity_flag_state_transition(0u);
    nav_selected_location_record = 0u;
    nav_actor_presentation_state = 10u;
    mouse_primary_pressed = 0u;
    transition_flags = 0u;
    line_completed = presentation_line_helper(line);
    if (line->frame_index == 7u) {
        if ((nav_camera_view_active & NAV_CAMERA_VIEW_ACTIVE_FLAG) == 0u) {
            transition_flags = (cb_u8)page_flip();
        }
        snd_play_clip(3);
        nav_camera_view_state = 8u;
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    }
    if (line_completed) {
        nav_actor_5_active = 0u;
        transition_flags = 7u;
        line->flags = transition_flags;
    }

transition_test:
    if ((transition_flags & NAV_ACTOR_TRANSITION_FLAG) == 0u) {
        return;
    }

    nav_camera_view_active ^= NAV_CAMERA_VIEW_ACTIVE_FLAG;
    vm_ui_flags &= (cb_u8)~BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    if ((nav_camera_view_active & NAV_CAMERA_VIEW_ACTIVE_FLAG) != 0u) {
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    } else {
        ship_3d_hud_palette_snapshot_and_camera_reset();
        nav_screen_rebuild_pending = 1u;
    }
    nav_location_panel_active = 0u;
    line->flags = NAV_ACTOR_PRESENT_FLAG;
    entity_flag_state_transition(4u);
}
