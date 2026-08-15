#include <dos.h>
#include <string.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_startup.h"
#include "../include/bloodprg_vm.h"

#define SHIP_3D_RECORD_REDIRECT_FLAG 0x0080u
#define SHIP_3D_CANDIDATE_UNRESTRICTED_FLAG 0x02u
#define SHIP_3D_NAVIGATION_RECORD_KIND 0x00c4u
#define SHIP_3D_RECORD_NAME_OFFSET 4u
#define SHIP_3D_INTERPOLATION_STEPS 6u
#define SHIP_3D_TRANSITION_PALETTE_BYTES 768u
#define SHIP_3D_SCENE_PALETTE_DWORD_OFFSET 0x60u

typedef struct ship_3d_navigation_record {
    cb_u16 kind;
    cb_u8 flags;
    cb_u8 reserved_03[17];
    cb_u16 access_count;
    cb_u8 reserved_16[2];
    cb_u16 relation;
} ship_3d_navigation_record;

typedef char ship_3d_navigation_record_relation_must_be_at_18[
        CB_OFFSETOF(ship_3d_navigation_record, relation) == 0x18 ? 1 : -1];

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define SHIP_3D_NAV_RECORD_AT(offset) \
    ((volatile ship_3d_navigation_record CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define SHIP_3D_NAV_RECORD_AT(offset) \
    ((volatile ship_3d_navigation_record CB_FAR *)(vm_record_base + (offset)))
#endif

void CB_NEAR ship_3d_navigation_update(void)
{
    volatile ship_3d_navigation_record CB_FAR *current_record;
    volatile ship_3d_navigation_record CB_FAR *counter_record;
    volatile ship_3d_navigation_record CB_FAR *candidate_record;
    const volatile cb_u16 CB_NEAR *candidate_cursor;
    cb_u16 current;
    cb_u16 candidate;
    cb_i16 selection;
    int accepted;
    int interpolation_complete;

    if ((ship_3d_navigation_trigger & 1u) != 0u) {
        nav_actor_presentation_state = presentation_mode_previous_state;
        current = ship_3d_current_target;
        current_record = SHIP_3D_NAV_RECORD_AT(current);
        counter_record = current_record;
        if ((current_record->kind & SHIP_3D_RECORD_REDIRECT_FLAG) != 0u) {
            counter_record = SHIP_3D_NAV_RECORD_AT(
                    current_record->access_count);
        }
        ++counter_record->access_count;

        ship_3d_navigation_candidate_build(
                (const volatile bloodprg_vm_object_header CB_FAR *)
                    current_record);
        candidate_cursor = ship_3d_navigation_candidate_offsets;
        accepted = 0;
        for (;;) {
            candidate = *candidate_cursor++;
            if (candidate == 0u) {
                break;
            }

            candidate_record = SHIP_3D_NAV_RECORD_AT(candidate);
            if ((current_record->flags
                    & SHIP_3D_CANDIDATE_UNRESTRICTED_FLAG) == 0u
                    && candidate_record->relation != current) {
                continue;
            }
            if (vm_named_ark_object != current
                    && candidate_record->relation == vm_named_ark_object) {
                break;
            }

            nav_deferred_record_type = SHIP_3D_NAVIGATION_RECORD_KIND;
            nav_deferred_record_link = candidate;
            (void)vm_c2_descript_lookup(
                    (const volatile cb_u8 CB_FAR *)candidate_record
                        + SHIP_3D_RECORD_NAME_OFFSET);
            accepted = 1;
            break;
        }

        if (!accepted) {
            vm_ui_flags |= SHIP_PRESENTATION_HUD;
            framebuffer_transition_current_step = 0u;
            framebuffer_transition_total_steps =
                    SHIP_3D_INTERPOLATION_STEPS;
            presentation_list_editing = 1u;
            (void)list_widget_layout_unified(
                    ship_3d_navigation_trigger_target_list);
            presentation_list_editing = 0u;
            presentation_word_choice_target_rect.x =
                    presentation_choice_current_rect[0];
            presentation_word_choice_target_rect.width =
                    presentation_choice_current_rect[2];
        }

        ship_3d_navigation_trigger = 0u;
        vm_sequence_active_ds = 1u;
        resource_vertical_offset = 0x0023u;
        vm_loaded_scene_image_path = (volatile char CB_NEAR *)0xffffu;

        graphics_band_top_row = 0x0023u;
        graphics_band_bottom_row = 0x00a5u;
        back_buffer_fill(0u);
        graphics_band_top_row = 0u;
        graphics_band_bottom_row = 200u;

        pbm_palette_refresh_ds = 1u;
        pbm_transparent_zero_ds = 1u;
        resource_force_write_directory = 1u;
        (void)pbm_image_load_and_decode(
                startup_transient_paths[0], graphics_back_buffer_ds);
        resource_force_write_directory = 0u;
        _fmemcpy(presentation_palette_dwords,
                &scene_palette_dwords[SHIP_3D_SCENE_PALETTE_DWORD_OFFSET],
                (cb_u16)sizeof(presentation_palette_dwords));
        pbm_palette_refresh_ds = 0u;
        pbm_transparent_zero_ds = 0u;

        vm_text_menu_pending = 0u;
        vm_text_selector = -1;
        ship_3d_depth_closing = 1u;
        ship_3d_depth_step = 2u;
        (void)palette_blend_remap_table_build(
                -50, 0u, 0u, 0u, graphics_span_remap_table);
    }

    if ((ship_3d_exit_pending & 1u) != 0u) {
        if ((ship_3d_depth_opening & 1u) == 0u) {
            goto reset_navigation;
        }
    } else if ((vm_sequence_active_ds & 1u) == 0u) {
        if ((vm_presentation_defer_a & 1u) != 0u) {
            return;
        }
        ship_3d_exit_pending = 1u;
        ship_3d_depth_opening = 1u;
        return;
    }

    alien_overlay_cycle();
    (void)bridge_steer_update(0);
    if ((vm_presentation_active & 1u) != 0u) {
        return;
    }

    full_screen_blit((const cb_u32 CB_FAR *)graphics_back_buffer_ds);
    resource_frame_presented = 1u;
    if (framebuffer_transition_total_steps ==
            SHIP_3D_INTERPOLATION_STEPS) {
        interpolation_complete = framebuffer_transition_current_step
                == framebuffer_transition_total_steps;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                &presentation_word_choice_target_rect);
        if (!interpolation_complete) {
            return;
        }

        selection = list_widget_layout_unified(
                ship_3d_navigation_trigger_target_list);
        if (selection < 0) {
            return;
        }
        vm_sequence_active_ds = 0u;
        ship_3d_exit_pending = 1u;
    }
    return;

reset_navigation:
    blit_fill_row_5221(0u);
    palette_scene_entries_clear();
    vm_ui_state.word = 9u;
    nav_bridge_seek_target_arc = 0u;
    bridge_seek_initial_distance = 0x0032u;
    nav_screen_rebuild_pending = 1u;
    ship_3d_navigation_snapshot_pending = 1u;
    vm_ship_active_flags = 0u;
    resource_vertical_offset = 0u;
    vm_text_selector = -1;
    vm_active_line = 0xffffu;
    vm_c2_presentation_gate = 0u;
    ship_3d_exit_pending = 0u;
    ship_3d_hud_initialized = 0u;
    vm_text_display_active = 0u;
    vm_presentation_defer_a = 0u;
    vm_presentation_hold_ready = 0u;
    ship_3d_plane_blit_crop_enabled_ds = 0u;
    vm_sequence_active_ds = 0u;
    vm_presentation_request_flags &= (cb_u8)~0x03u;
    vm_presentation_word_choice_phase = 0u;
    (void)back_buffer_init();
    ship_3d_hud_palette_snapshot_and_camera_reset();
    _fmemcpy(palette_transition_source_gs,
            bridge_panorama_palette,
            0x0240u);
    _fmemset(palette_transition_target,
            0,
            SHIP_3D_TRANSITION_PALETTE_BYTES);
    palette_transition_last = 0xffu;
    palette_transition_percent = 0u;
    palette_transition_increment = 10u;
}

#undef SHIP_3D_NAV_RECORD_AT
