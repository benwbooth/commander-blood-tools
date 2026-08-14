/* Codegen probe for BLOODPRG 0x001855. */

#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

#define TRANSITION_ACTIVE 0x01u
#define TRANSITION_LOAD 0x02u
#define TRANSITION_DEFERRED 0x04u
#define TRANSITION_BRIDGE 0x08u
#define TRANSITION_FINISH 0x10u
#define TRANSITION_RELOAD 0x40u
#define TRANSITION_BLOCKED 0x80u
#define PRESENTATION_RECORD_KIND 2u
#define HIGH_PALETTE_OFFSET 384u
#define HIGH_PALETTE_BYTES 192u

typedef volatile u8 FAR *buffer_ptr;

typedef union ui_state_probe {
    u16 word;
    struct {
        u8 flags;
        u8 auxiliary;
    } bytes;
} ui_state_probe;

typedef struct record_triple_probe {
    u16 kind;
    u16 related;
    u16 value;
} record_triple_probe;

extern volatile u8 transition_phase_probe;
extern volatile u16 clip_snapshot_flags_probe;
extern volatile ui_state_probe ui_state_probe_value;
extern volatile u16 active_line_probe;
extern volatile u16 scene_record_offset_probe;
extern volatile u16 deferred_record_link_probe;
extern volatile u16 deferred_record_type_probe;
extern volatile record_triple_probe FAR *record_base_probe;
extern volatile u8 c2_gate_probe;
extern volatile u8 scene_gate_probe;
extern volatile u16 resource_vertical_offset_probe;
extern volatile u8 palette_refresh_probe;
extern volatile u8 transparent_zero_probe;
extern volatile char scene_image_path_probe[];
extern buffer_ptr graphics_back_buffer_probe;
extern volatile u16 animation_selector_probe;
extern volatile u16 band_top_probe;
extern volatile u16 band_bottom_probe;
extern volatile u8 live_palette_probe[768];
extern u8 palette_target_probe[768];
extern u8 palette_source_probe[768];
extern volatile u8 palette_first_probe;
extern volatile u8 palette_last_probe;
extern volatile u16 palette_increment_probe;
extern volatile u16 palette_percent_probe;
extern volatile u8 presentation_active_probe;
extern volatile int text_selector_probe;
extern volatile u8 text_display_active_probe;
extern volatile u8 presentation_defer_probe;
extern volatile u8 presentation_hold_probe;
extern volatile u8 presentation_request_probe;
extern volatile u8 presentation_text_wait_probe;
extern volatile u8 redraw_pending_probe;

extern void FAR entity_transition_probe(u16 object_id);
extern u16 FAR descript_lookup_probe(
        const volatile u8 FAR *record_name);
extern void FAR scene_dispatch_probe(u16 phase);
extern int FAR image_load_probe(
        volatile char FAR *path, volatile u8 FAR *destination);
extern void FAR full_screen_blit_probe(const volatile u8 FAR *source);
extern void FAR back_buffer_fill_probe(u8 color);
extern void FAR bridge_update_probe(void);
extern void FAR alien_overlay_probe(void);
extern void FAR hud_palette_camera_reset_probe(void);

#define RECORD_AT(offset) \
    ((volatile record_triple_probe FAR *) \
        MK_FP(FP_SEG(record_base_probe), (offset)))

void NEAR scene_transition_step_probe(u16 link_target_offset)
{
    volatile record_triple_probe FAR *record;
    u16 index;
    u8 component;
    u8 phase;

    phase = transition_phase_probe;
    if ((phase & TRANSITION_ACTIVE) == 0u) {
        return;
    }

    clip_snapshot_flags_probe = 1u;
    if ((phase & (u8)~TRANSITION_ACTIVE) == 0u) {
        entity_transition_probe(4u);
        entity_transition_probe(31u);
        ui_state_probe_value.word = 0u;
        transition_phase_probe |= TRANSITION_LOAD;
        active_line_probe = 0x0029u;
        scene_record_offset_probe = deferred_record_link_probe;
        (void)descript_lookup_probe(
                (const volatile u8 FAR *)RECORD_AT(
                    (u16)(scene_record_offset_probe + 4u)));
        return;
    }

    scene_dispatch_probe(link_target_offset);
    if ((phase & TRANSITION_LOAD) != 0u) {
        if ((c2_gate_probe & 1u) != 0u) {
            return;
        }
        transition_phase_probe = 5u;
        resource_vertical_offset_probe = 0x0023u;
        scene_gate_probe = 1u;
        palette_refresh_probe = 1u;
        transparent_zero_probe = 0u;
        (void)image_load_probe(
                scene_image_path_probe, graphics_back_buffer_probe);
        full_screen_blit_probe(graphics_back_buffer_probe);

        record = RECORD_AT(scene_record_offset_probe);
        if (record->kind != PRESENTATION_RECORD_KIND) {
            animation_selector_probe = 0xffffu;
            band_top_probe = 0x0023u;
            band_bottom_probe = 0x00a5u;
            back_buffer_fill_probe(0u);
            band_top_probe = 0u;
            band_bottom_probe = 200u;
            transition_phase_probe = 9u;
            active_line_probe = 0x002bu;
            return;
        }

        for (index = 0u; index < HIGH_PALETTE_BYTES; ++index) {
            palette_target_probe[HIGH_PALETTE_OFFSET + index] =
                    live_palette_probe[HIGH_PALETTE_OFFSET + index];
        }
        for (index = 0u; index < HIGH_PALETTE_BYTES; ++index) {
            component = live_palette_probe[HIGH_PALETTE_OFFSET + index];
            palette_source_probe[HIGH_PALETTE_OFFSET + index] =
                    component < 40u ? 0u : (u8)(component - 40u);
        }
        palette_first_probe = 0x80u;
        palette_last_probe = 0xbfu;
        palette_increment_probe = 5u;
        active_line_probe = 0x0027u;
        return;
    }

    if ((phase & TRANSITION_DEFERRED) != 0u) {
        if ((c2_gate_probe & 1u) != 0u) {
            return;
        }
        deferred_record_type_probe = 0x00c4u;
        transition_phase_probe = 0x89u;
        animation_selector_probe = 0u;
        return;
    }

    if ((phase & TRANSITION_BRIDGE) != 0u) {
        bridge_update_probe();
        record = RECORD_AT(scene_record_offset_probe);
        if (record->kind != PRESENTATION_RECORD_KIND) {
            if ((c2_gate_probe & 1u) != 0u) {
                return;
            }
            resource_vertical_offset_probe = 0u;
            transition_phase_probe = 0x21u;
            active_line_probe = 0x002au;
            scene_gate_probe = 0u;
            return;
        }
        if ((transition_phase_probe & TRANSITION_BLOCKED) != 0u) {
            return;
        }
        if (active_line_probe == 7u) {
            transition_phase_probe |= TRANSITION_RELOAD;
            return;
        }
        if ((transition_phase_probe & TRANSITION_RELOAD) != 0u) {
            transition_phase_probe &= (u8)~TRANSITION_RELOAD;
            palette_refresh_probe = 0u;
            (void)image_load_probe(
                    scene_image_path_probe, graphics_back_buffer_probe);
            return;
        }

        alien_overlay_probe();
        if ((presentation_active_probe & 1u) != 0u
                || (c2_gate_probe & 1u) != 0u) {
            return;
        }
        transition_phase_probe = 0x11u;
        active_line_probe = 0x0028u;
        for (index = 0u; index < HIGH_PALETTE_BYTES; ++index) {
            palette_source_probe[HIGH_PALETTE_OFFSET + index] =
                    palette_target_probe[HIGH_PALETTE_OFFSET + index];
        }
        for (index = 0u; index < HIGH_PALETTE_BYTES; ++index) {
            palette_target_probe[HIGH_PALETTE_OFFSET + index] =
                    live_palette_probe[HIGH_PALETTE_OFFSET + index];
        }
        palette_percent_probe = 0u;
        return;
    }

    if ((phase & TRANSITION_FINISH) != 0u) {
        if ((c2_gate_probe & 1u) != 0u) {
            return;
        }
        resource_vertical_offset_probe = 0u;
        transition_phase_probe = 0x21u;
        active_line_probe = 0x002au;
        scene_gate_probe = 0u;
        return;
    }

    if ((c2_gate_probe & 1u) != 0u) {
        return;
    }
    animation_selector_probe = 0u;
    transition_phase_probe = 0u;
    ui_state_probe_value.word = 1u;
    text_selector_probe = -1;
    active_line_probe = 0xffffu;
    c2_gate_probe = 0u;
    text_display_active_probe = 0u;
    presentation_defer_probe = 0u;
    presentation_hold_probe = 0u;
    presentation_request_probe &= (u8)~3u;
    presentation_text_wait_probe = 0u;
    redraw_pending_probe = 1u;
    hud_palette_camera_reset_probe();
}
