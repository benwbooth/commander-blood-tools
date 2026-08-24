#include <dos.h>
#include <string.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define SHIP_3D_RECORD_LINK_OFFSET 0x0016u
#define SHIP_3D_RECORD_PROBE_MASK 0x0140u
#define SHIP_3D_TARGET_NAME_BYTES 4u
#define SHIP_3D_C1_COMMAND_OFFSET 0x000au
#define SHIP_3D_C1_COMMAND_KIND 0x00c1u
#define SHIP_3D_LAST_ENTITY_ID 31u
#define SHIP_3D_PALETTE_CLEAR_BYTES 0x0240u
#define SHIP_3D_PALETTE_TAIL_BYTES 0x0040u

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define SHIP_3D_RECORD_AT(offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define SHIP_3D_RECORD_AT(offset) (vm_record_base + (offset))
#endif

void CB_NEAR ship_3d_hud_init(void)
{
    volatile cb_u8 CB_FAR *record;
    volatile bloodprg_vm_record_triple CB_FAR *command;
    bloodprg_graphics_buffer_ptr volatile saved_framebuffer;
    cb_u16 target;

    if ((ship_3d_hud_initialized & 1u) == 0u) {
        if ((ship_3d_hud_init_pending & 1u) != 0u) {
            ship_3d_hud_init_pending = 0u;
            vm_subtitle_display_mode_ds = 0u;
            _fmemcpy(ship_3d_hud_palette_stage_dwords,
                    ship_3d_hud_pyramid_palette_dwords,
                    SHIP_3D_HUD_PALETTE_BYTES);
        }

        (void)backbuffer_clear_flags();
        nav_bridge_seek_target_arc = 0u;
        vm_bridge_view_frame = 0x00b3;
        vm_ui_state.word |= 8u;
        manu3_animation_selector_request = 1u;
        ship_3d_hud_initialized = 1u;
        vm_state_record_processor();

        ship_3d_target_select_phase = 1u;
        ship_3d_target_layout_center_x = 0x0050u;
        ship_3d_target_layout_preserve_widths = 1u;
        ship_3d_target_layout_extra_entry = 1u;
        ship_3d_interpolation_duration = 10u;

        record = SHIP_3D_RECORD_AT(vm_arche_record_offset);
        (void)ship_3d_presentable_name_list_build(
                (const volatile bloodprg_vm_object_header CB_FAR *)record);
        target = *(volatile cb_u16 CB_FAR *)(
                record + SHIP_3D_RECORD_LINK_OFFSET);
        /* The binary probes ES:EAX here, but the link and every later record
         * access are 16-bit offsets. */
        if ((*(volatile cb_u16 CB_FAR *)SHIP_3D_RECORD_AT(target)
                & SHIP_3D_RECORD_PROBE_MASK) == 0u) {
            record = SHIP_3D_RECORD_AT(target);
            (void)ship_3d_presentable_name_list_build(
                    (const volatile bloodprg_vm_object_header CB_FAR *)record);
            ship_3d_current_target = target;
        } else {
            ship_3d_current_target = (cb_u16)(
                    ship_3d_presentable_name_offsets[0]
                    - SHIP_3D_TARGET_NAME_BYTES);
        }
        (void)vm_c2_descript_lookup(
                SHIP_3D_RECORD_AT(
                    (cb_u16)(ship_3d_current_target
                        + SHIP_3D_TARGET_NAME_BYTES)));

        ship_3d_scene_dispatch_blocked = 1u;
        vm_active_line = 3u;
        ship_3d_plane_blit_crop_enabled_ds = 1u;
        resource_vertical_offset = byte_parser_word_1fa5;
        vm_c2_presentation_gate = 0u;
        dlg_line_id_scene_dispatch(byte_parser_word_1fa5);
        fullscreen_copy_to_backbuffer_far(
                (const cb_u32 CB_FAR *)graphics_display_buffer_ds);
        ship_3d_plane_band_copy();

        _fmemcpy(palette_transition_source_gs,
                (const void CB_FAR *)live_palette, 768u);
        _fmemset(palette_transition_target, 0, SHIP_3D_PALETTE_CLEAR_BYTES);
        _fmemcpy(palette_transition_target + SHIP_3D_PALETTE_CLEAR_BYTES,
                ship_3d_hud_palette_stage_dwords,
                SHIP_3D_PALETTE_TAIL_BYTES);
        palette_transition_percent = 0u;
        palette_transition_increment = 10u;
        palette_transition_first = 0u;
        palette_transition_last = 0xc0u;
    }

    if ((ship_3d_exit_pending & 1u) != 0u) {
        goto close_presentation;
    }

    (void)bridge_steer_update(0);
    if ((vm_ui_state.word & 8u) != 0u) {
        bloodprg_clip_bounds.top = 0x0023u;
        bloodprg_clip_bounds.bottom = 0x00a5u;
        (void)palette_blend_remap_table_build(
                (cb_i16)-50, 0u, 0u, 0u,
                graphics_span_remap_table);
        bloodprg_clip_bounds.top = 0u;
        bloodprg_clip_bounds.bottom = 200u;
    }

    bloodprg_clip_snapshot_flags = 1u;
    sprite_slot_commit_dirty_range(0u, SHIP_3D_LAST_ENTITY_ID);
    dirty_rects_copy_secondary_to_primary(
            (const volatile bloodprg_dirty_rect CB_FAR *)
                &bloodprg_dirty_rect_list[0]);
    if ((vm_text_display_active & 1u) == 0u) {
        return;
    }
    if (*(volatile cb_u8 CB_NEAR *)vm_text_reveal_cursor != 0u) {
        return;
    }

    if (palette_transition_percent == 100u
            && palette_transition_increment == 10u) {
        palette_transition_increment = 0u;
    }
    resource_frame_presented = 1u;
    target = ship_3d_target_record_select();
    if (target == 0u) {
        return;
    }
    if (target == 0xffffu) {
        goto close_presentation;
    }

    if (target != ship_3d_current_target) {
        ship_3d_current_target = target;
        (void)vm_c2_descript_lookup(
                SHIP_3D_RECORD_AT(
                    (cb_u16)(target + SHIP_3D_TARGET_NAME_BYTES)));
    }

    if ((snd_music_voc_name_changed & 1u) != 0u) {
        saved_framebuffer = graphics_draw_framebuffer_ds;
        graphics_draw_framebuffer_ds = graphics_screen_buffer_ds;
        ship_3d_plane_band_copy();
        graphics_draw_framebuffer_ds = saved_framebuffer;
        snd_driver_call();
        snd_stream_source_load(presentation_music_voc_path);
    }
    snd_stream_start();

    command = (volatile bloodprg_vm_record_triple CB_FAR *)
            SHIP_3D_RECORD_AT(
                (cb_u16)(vm_named_orxx_object + SHIP_3D_C1_COMMAND_OFFSET));
    command->kind = SHIP_3D_C1_COMMAND_KIND;
    command->related = target;
    command->value = 0u;
    ship_3d_scene_dispatch_blocked = 0u;
    return;

close_presentation:
    ship_3d_exit_pending = 1u;
    if ((ship_3d_depth_opening & 1u) != 0u) {
        return;
    }
    vm_ship_active_flags = 0x0011u;
    vm_sequence_active_ds = 0u;
    vm_text_display_active = 0u;
    ship_3d_scene_dispatch_blocked = 0u;
    vm_bridge_redraw_pending = 0u;
    ship_3d_exit_pending = 0u;
}

#undef SHIP_3D_RECORD_AT
