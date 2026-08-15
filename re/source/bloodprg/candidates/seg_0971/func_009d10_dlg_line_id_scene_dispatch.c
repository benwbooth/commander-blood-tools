#include <dos.h>
#include <string.h>

#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define NO_SCENE_IMAGE ((volatile char CB_NEAR *)0xffffu)
#define SCENE_PALETTE_DWORD_OFFSET 0x60u
#define SHIP_ACTIVE_PRESENTATION_MASK 0x08u
#define PRESENTATION_REQUEST_MASK 0x02u

void CB_FAR dlg_line_id_scene_dispatch(cb_u16 link_target_offset)
{
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile char CB_NEAR *image_path;
    cb_u16 line;
    cb_u16 index;

    resource_frame_presented = 0u;
    line = vm_active_line;
    if ((cb_i16)line < 0) {
        return;
    }

    if ((vm_c2_presentation_gate & 1u) == 0u) {
        if (line == 0x001du) {
            record = (volatile bloodprg_vm_record_triple CB_FAR *)MK_FP(
                    FP_SEG(vm_record_base),
                    vm_primary_c4_record);
            ship_3d_alien_overlay_armed =
                    record->related == vm_named_scruter_jo_object;
        } else if ((ship_3d_alien_overlay_armed & 1u) != 0u) {
            ship_3d_temp_snd_trigger = 1u;
            return;
        }

        if ((vm_scene_gate & 1u) == 0u) {
            image_path = resource_index[line].image_path;
            if (image_path == NO_SCENE_IMAGE) {
                vm_loaded_scene_image_path = NO_SCENE_IMAGE;
            } else if (image_path != vm_loaded_scene_image_path) {
                vm_loaded_scene_image_path = image_path;
                pbm_palette_refresh = 1u;
                pbm_transparent_zero = 1u;
                resource_force_write_directory = 1u;
                (void)pbm_image_load_and_decode(
                        image_path, graphics_back_buffer);
                resource_force_write_directory = 0u;
                pbm_palette_refresh = 0u;
                pbm_transparent_zero = 0u;
                _fmemcpy(
                        (void CB_FAR *)presentation_palette_dwords,
                        (const void CB_FAR *)&scene_palette_dwords[
                                SCENE_PALETTE_DWORD_OFFSET],
                        (cb_u16)sizeof(presentation_palette_dwords));
            }

            if (vm_loaded_scene_image_path == NO_SCENE_IMAGE) {
                graphics_band_top_row = resource_vertical_offset;
                graphics_band_bottom_row =
                        (cb_u16)(resource_vertical_offset + 0x0082u);
                back_buffer_fill(0u);
                graphics_band_top_row = 0u;
                graphics_band_bottom_row = 200u;
            }
        }

        vm_c2_presentation_gate = 1u;
        resource_draw_via_back_buffer = 0u;
        resource_skip_back_buffer_present = 0u;
        resource_unclamped_row_count = 0u;
        for (index = 0u; index < 8u; ++index) {
            if (presentation_unclamped_line_ids[index] == (cb_u8)line) {
                resource_unclamped_row_count = 1u;
                break;
            }
        }

        if (line == 2u || line == 7u) {
            resource_draw_via_back_buffer = 1u;
        } else {
            switch (line) {
            case 0u:
            case 1u:
            case 3u:
            case 4u:
            case 5u:
            case 6u:
            case 0x0029u:
            case 0x002au:
            case 0x002bu:
            case 0x002cu:
                vm_presentation_request_flags |= PRESENTATION_REQUEST_MASK;
                resource_skip_back_buffer_present = 1u;
                break;
            }
        }

        resource_source_is_banked = 0u;
        if (line == 8u
                && (cb_u16)(resource_xms_handle + resource_ems_handle)
                        != 0xfffeu) {
            resource_source_is_banked = 1u;
        }

        resource_load_sequence(line);
        if ((vm_sequence_active | vm_scene_gate) == 0u) {
            return;
        }

        line = vm_active_line;
        if (line == vm_displayed_line) {
            return;
        }
        vm_displayed_line = line;
        (void)palette_blend_remap_table_build(
                -50, 0u, 0u, 0u, graphics_span_remap_table);
        return;
    }

    if ((ship_3d_scene_dispatch_blocked & 1u) != 0u) {
        return;
    }
    ems_resource_flush(link_target_offset);
    if (!list_d8c_state_le_one()) {
        if ((vm_ship_active_flags_low & SHIP_ACTIVE_PRESENTATION_MASK) != 0u) {
            vm_bridge_redraw_pending = 1u;
        }
        if (vm_active_line == 5u) {
            graphics_band_top_row = 0x0023u;
            graphics_band_bottom_row = 0x00a5u;
            blit_fill_row_5221(0u);
            graphics_band_top_row = 0u;
            graphics_band_bottom_row = 200u;
        }
        ship_3d_temp_snd_trigger =
                (ship_3d_alien_overlay_armed & 1u) != 0u;
        ship_3d_nav_choice_sound_gate =
                (vm_finale_requested & 1u) != 0u;
        vm_c2_presentation_gate = 0u;
        vm_displayed_line = vm_active_line;
        vm_active_line = 0xffffu;
        vm_presentation_request_flags &= (cb_u8)~PRESENTATION_REQUEST_MASK;
        return;
    }

    if (vm_active_line == 0x0027u) {
        if ((cb_u16)(list_d8c_entry_metric - list_d8c_read_wrap_index)
                == 0x0014u) {
            palette_transition_percent = 0u;
        }
    } else if ((vm_ship_active_flags_low & SHIP_ACTIVE_PRESENTATION_MASK) != 0u
            && (cb_u16)(list_d8c_entry_metric - list_d8c_read_wrap_index)
                    == 8u) {
        ship_3d_depth_opening = 1u;
        ship_3d_depth_step = 6u;
    }
}
