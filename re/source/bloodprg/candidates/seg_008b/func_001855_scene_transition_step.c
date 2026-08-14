#include <dos.h>

#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define SCENE_TRANSITION_ACTIVE 0x01u
#define SCENE_TRANSITION_LOAD 0x02u
#define SCENE_TRANSITION_DEFERRED_RECORD 0x04u
#define SCENE_TRANSITION_BRIDGE 0x08u
#define SCENE_TRANSITION_FINISH 0x10u
#define SCENE_TRANSITION_RELOAD 0x40u
#define SCENE_TRANSITION_BLOCKED 0x80u

#define SCENE_RECORD_PRESENTATION_KIND 2u
#define SCENE_HIGH_PALETTE_OFFSET 384u
#define SCENE_HIGH_PALETTE_BYTES 192u

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define SCENE_RECORD_AT(offset) \
    ((volatile bloodprg_vm_record_triple CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define SCENE_RECORD_AT(offset) \
    ((volatile bloodprg_vm_record_triple CB_FAR *) \
        (vm_record_base + (offset)))
#endif

void CB_NEAR scene_transition_step(cb_u16 link_target_offset)
{
    volatile bloodprg_vm_record_triple CB_FAR *record;
    cb_u16 index;
    cb_u8 component;
    cb_u8 phase;

    phase = render_update_flag_2751;
    if ((phase & SCENE_TRANSITION_ACTIVE) == 0u) {
        return;
    }

    bloodprg_clip_snapshot_flags = 1u;
    if ((phase & (cb_u8)~SCENE_TRANSITION_ACTIVE) == 0u) {
        entity_flag_state_transition(4u);
        entity_flag_state_transition(31u);
        vm_ui_state.word = 0u;
        render_update_flag_2751 |= SCENE_TRANSITION_LOAD;
        vm_active_line = 0x0029u;
        vm_scene_record_offset = nav_deferred_record_link;
        (void)vm_c2_descript_lookup(
                (const volatile cb_u8 CB_FAR *)SCENE_RECORD_AT(
                    (cb_u16)(vm_scene_record_offset + 4u)));
        return;
    }

    dlg_line_id_scene_dispatch(link_target_offset);
    if ((phase & SCENE_TRANSITION_LOAD) != 0u) {
        if ((vm_c2_presentation_gate & 1u) != 0u) {
            return;
        }

        render_update_flag_2751 = 5u;
        resource_vertical_offset = 0x0023u;
        vm_scene_gate = 1u;
        pbm_palette_refresh = 1u;
        pbm_transparent_zero = 0u;
        (void)pbm_image_load_and_decode(
                scene_transition_image_path, graphics_back_buffer);
        full_screen_blit((const cb_u32 CB_FAR *)graphics_back_buffer);

        record = SCENE_RECORD_AT(vm_scene_record_offset);
        if (record->kind != SCENE_RECORD_PRESENTATION_KIND) {
            manu3_animation_selector_request = 0xffffu;
            graphics_band_top_row = 0x0023u;
            graphics_band_bottom_row = 0x00a5u;
            back_buffer_fill(0u);
            graphics_band_top_row = 0u;
            graphics_band_bottom_row = 200u;
            render_update_flag_2751 = 9u;
            vm_active_line = 0x002bu;
            return;
        }

        for (index = 0u; index < SCENE_HIGH_PALETTE_BYTES; ++index) {
            palette_transition_target[SCENE_HIGH_PALETTE_OFFSET + index] =
                    pbm_live_palette[SCENE_HIGH_PALETTE_OFFSET + index];
        }
        for (index = 0u; index < SCENE_HIGH_PALETTE_BYTES; ++index) {
            component =
                    pbm_live_palette[SCENE_HIGH_PALETTE_OFFSET + index];
            palette_transition_source[SCENE_HIGH_PALETTE_OFFSET + index] =
                    component < 40u ? 0u : (cb_u8)(component - 40u);
        }
        palette_transition_first = 0x80u;
        palette_transition_last = 0xbfu;
        palette_transition_increment = 5u;
        vm_active_line = 0x0027u;
        return;
    }

    if ((phase & SCENE_TRANSITION_DEFERRED_RECORD) != 0u) {
        if ((vm_c2_presentation_gate & 1u) != 0u) {
            return;
        }
        nav_deferred_record_type = 0x00c4u;
        render_update_flag_2751 = 0x89u;
        manu3_animation_selector_request = 0u;
        return;
    }

    if ((phase & SCENE_TRANSITION_BRIDGE) != 0u) {
        (void)bridge_steer_update((cb_u16 CB_NEAR *)0);
        record = SCENE_RECORD_AT(vm_scene_record_offset);
        if (record->kind != SCENE_RECORD_PRESENTATION_KIND) {
            if ((vm_c2_presentation_gate & 1u) != 0u) {
                return;
            }
            resource_vertical_offset = 0u;
            render_update_flag_2751 = 0x21u;
            vm_active_line = 0x002au;
            vm_scene_gate = 0u;
            return;
        }

        if ((render_update_flag_2751 & SCENE_TRANSITION_BLOCKED) != 0u) {
            return;
        }
        if (vm_active_line == 7u) {
            render_update_flag_2751 |= SCENE_TRANSITION_RELOAD;
            return;
        }
        if ((render_update_flag_2751 & SCENE_TRANSITION_RELOAD) != 0u) {
            render_update_flag_2751 &=
                    (cb_u8)~SCENE_TRANSITION_RELOAD;
            pbm_palette_refresh = 0u;
            (void)pbm_image_load_and_decode(
                    scene_transition_image_path, graphics_back_buffer);
            return;
        }

        alien_overlay_cycle();
        if ((vm_presentation_active & 1u) != 0u
                || (vm_c2_presentation_gate & 1u) != 0u) {
            return;
        }

        render_update_flag_2751 = 0x11u;
        vm_active_line = 0x0028u;
        for (index = 0u; index < SCENE_HIGH_PALETTE_BYTES; ++index) {
            palette_transition_source[SCENE_HIGH_PALETTE_OFFSET + index] =
                    palette_transition_target[
                        SCENE_HIGH_PALETTE_OFFSET + index];
        }
        for (index = 0u; index < SCENE_HIGH_PALETTE_BYTES; ++index) {
            palette_transition_target[SCENE_HIGH_PALETTE_OFFSET + index] =
                    pbm_live_palette[SCENE_HIGH_PALETTE_OFFSET + index];
        }
        palette_transition_percent = 0u;
        return;
    }

    if ((phase & SCENE_TRANSITION_FINISH) != 0u) {
        if ((vm_c2_presentation_gate & 1u) != 0u) {
            return;
        }
        resource_vertical_offset = 0u;
        render_update_flag_2751 = 0x21u;
        vm_active_line = 0x002au;
        vm_scene_gate = 0u;
        return;
    }

    if ((vm_c2_presentation_gate & 1u) != 0u) {
        return;
    }
    manu3_animation_selector_request = 0u;
    render_update_flag_2751 = 0u;
    vm_ui_state.word = 1u;
    vm_text_selector = -1;
    vm_active_line = 0xffffu;
    vm_c2_presentation_gate = 0u;
    vm_text_display_active = 0u;
    vm_presentation_defer_a = 0u;
    vm_presentation_hold_ready = 0u;
    vm_presentation_request_flags &= (cb_u8)~3u;
    vm_presentation_text_wait = 0u;
    save_load_redraw_pending = 1u;
    ship_3d_hud_palette_snapshot_and_camera_reset();
}
