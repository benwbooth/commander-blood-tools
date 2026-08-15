#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

void CB_FAR ship_presentation_fsm(cb_u16 link_target_offset)
{
    cb_u16 state;
    cb_u16 line;

    state = vm_ship_active_flags;
    if ((state & SHIP_PRESENTATION_ACTIVE) == 0u) {
        return;
    }

    bloodprg_clip_snapshot_flags = 1u;
    if ((state & SHIP_PRESENTATION_PHASE_MASK) == 0u) {
        entity_flag_state_transition(4u);
        entity_flag_state_transition(31u);
        vm_ui_state.word = 0u;
        vm_ship_active_flags_low |= SHIP_PRESENTATION_DIALOGUE;
        ship_3d_dialogue_cycle_line = 4u;
        ship_3d_scene_dispatch_blocked = 0u;
        ship_3d_depth_offset = 0u;
        ship_3d_depth_opening = 0u;
        return;
    }

    ship_3d_depth_scroll_step();
    ship_3d_plane_band_copy();
    dlg_line_id_scene_dispatch(link_target_offset);

    if ((state & SHIP_PRESENTATION_DIALOGUE) != 0u) {
        if ((ship_3d_dialogue_phase_ready & 1u) == 0u) {
            if ((vm_c2_presentation_gate & 1u) != 0u) {
                return;
            }

            line = ship_3d_dialogue_cycle_line;
            if (line != 0u) {
                vm_active_line = line;
                ++line;
                if (line == SHIP_PRESENTATION_DIALOGUE_LINE_END) {
                    line = 0u;
                }
                ship_3d_dialogue_cycle_line = line;
                return;
            }

            ship_3d_dialogue_phase_ready = 0u;
            vm_ship_active_flags = 5u;
            return;
        }

        ship_3d_dialogue_phase_ready = 0u;
        vm_ship_active_flags = 5u;
    }

    if ((state & SHIP_PRESENTATION_HUD) != 0u) {
        if ((ship_3d_hud_init_pending & 1u) == 0u
                || palette_transition_percent
                        == SHIP_PRESENTATION_TRANSITION_COMPLETE) {
            ship_3d_hud_init();
        }
        return;
    }

    if ((state & SHIP_PRESENTATION_TRAVEL) != 0u) {
        if ((vm_bridge_redraw_pending & 1u) != 0u) {
            vm_ship_active_flags = 0x0011u;
            blit_fill_row_5221(0u);
        } else if ((vm_c2_presentation_gate & 1u) == 0u) {
            vm_active_line = 3u;
            vm_bridge_redraw_pending = 0u;
        }
        return;
    }

    if ((state & SHIP_PRESENTATION_NAVIGATION) != 0u) {
        ship_3d_navigation_update();
    }
}
