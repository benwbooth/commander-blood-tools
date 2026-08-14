/*
 * Codegen probe for BLOODPRG 0x00AFA0.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define ACTIVE 0x0001u
#define DIALOGUE 0x0002u
#define HUD 0x0004u
#define TRAVEL 0x0008u
#define NAVIGATION 0x0010u
#define PHASE_MASK 0x001eu

extern volatile u16 ship_state_probe;
extern volatile u16 clip_snapshot_probe;
extern volatile u16 ui_state_probe;
extern volatile u16 dialogue_cycle_probe;
extern volatile u8 scene_dispatch_blocked_probe;
extern volatile u16 depth_offset_probe;
extern volatile u8 depth_opening_probe;
extern volatile u8 dialogue_ready_probe;
extern volatile u8 presentation_gate_probe;
extern volatile u16 active_line_probe;
extern volatile u8 hud_pending_probe;
extern volatile u16 transition_percent_probe;
extern volatile u8 redraw_pending_probe;

void FAR entity_transition_probe(u16 object_id);
void NEAR depth_scroll_probe(void);
void FAR plane_band_probe(void);
void FAR scene_dispatch_probe(u16 state);
void NEAR hud_init_probe(void);
void FAR fill_display_band_probe(u8 color);
void NEAR navigation_update_probe(void);

#if defined(__WATCOMC__)
#pragma aux entity_transition_probe parm [ax] modify exact []
#pragma aux depth_scroll_probe modify exact []
#pragma aux plane_band_probe modify exact []
#pragma aux scene_dispatch_probe parm [ax] modify exact []
#pragma aux hud_init_probe modify exact []
#pragma aux fill_display_band_probe parm [ax] modify exact []
#pragma aux navigation_update_probe modify exact []
#endif

void FAR ship_presentation_fsm_probe(void)
{
    u16 state;
    u16 line;

    state = ship_state_probe;
    if ((state & ACTIVE) == 0u) {
        return;
    }

    clip_snapshot_probe = 1u;
    if ((state & PHASE_MASK) == 0u) {
        entity_transition_probe(4u);
        entity_transition_probe(31u);
        ui_state_probe = 0u;
        *(volatile u8 *)&ship_state_probe |= DIALOGUE;
        dialogue_cycle_probe = 4u;
        scene_dispatch_blocked_probe = 0u;
        depth_offset_probe = 0u;
        depth_opening_probe = 0u;
        return;
    }

    depth_scroll_probe();
    plane_band_probe();
    scene_dispatch_probe(state);

    if ((state & DIALOGUE) != 0u) {
        if ((dialogue_ready_probe & 1u) == 0u) {
            if ((presentation_gate_probe & 1u) != 0u) {
                return;
            }
            line = dialogue_cycle_probe;
            if (line != 0u) {
                active_line_probe = line;
                ++line;
                if (line == 6u) {
                    line = 0u;
                }
                dialogue_cycle_probe = line;
                return;
            }
            dialogue_ready_probe = 0u;
            ship_state_probe = 5u;
            return;
        }
        dialogue_ready_probe = 0u;
        ship_state_probe = 5u;
    }

    if ((state & HUD) != 0u) {
        if ((hud_pending_probe & 1u) == 0u
                || transition_percent_probe == 100u) {
            hud_init_probe();
        }
        return;
    }

    if ((state & TRAVEL) != 0u) {
        if ((redraw_pending_probe & 1u) != 0u) {
            ship_state_probe = 0x0011u;
            fill_display_band_probe(0u);
        } else if ((presentation_gate_probe & 1u) == 0u) {
            active_line_probe = 3u;
            redraw_pending_probe = 0u;
        }
        return;
    }

    if ((state & NAVIGATION) != 0u) {
        navigation_update_probe();
    }
}
