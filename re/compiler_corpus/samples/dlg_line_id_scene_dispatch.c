/* Codegen probe for BLOODPRG 0x009D10. */
#include <dos.h>
#include <string.h>

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#define FAR far
#define NEAR near
#define NO_SCENE_IMAGE ((volatile char NEAR *)0xffffu)

typedef struct vm_record_triple_probe {
    u16 kind;
    u16 related;
    u16 value;
} vm_record_triple_probe;

typedef struct resource_index_entry_probe {
    void NEAR *descriptor;
    volatile char NEAR *image_path;
} resource_index_entry_probe;

extern volatile u8 resource_frame_presented_probe;
extern volatile u16 vm_active_line_probe;
extern volatile u8 vm_c2_presentation_gate_probe;
extern volatile u8 FAR *vm_record_base_probe;
extern volatile u16 vm_primary_c4_record_probe;
extern volatile u16 vm_named_scruter_probe;
extern volatile u8 alien_overlay_armed_probe;
extern volatile u8 temp_snd_trigger_probe;
extern volatile u8 vm_scene_gate_probe;
extern volatile resource_index_entry_probe resource_index_probe[];
extern volatile char NEAR * volatile loaded_scene_image_path_probe;
extern volatile u8 pbm_palette_refresh_probe;
extern volatile u8 pbm_transparent_zero_probe;
extern volatile u8 force_directory_probe;
extern volatile u8 FAR *graphics_back_buffer_probe;
extern u32 scene_palette_dwords_probe[0x90];
extern u32 presentation_palette_dwords_probe[0x30];
extern volatile u16 graphics_band_top_probe;
extern volatile u16 graphics_band_bottom_probe;
extern volatile u16 resource_vertical_offset_probe;
extern volatile u8 draw_via_back_buffer_probe;
extern volatile u8 skip_back_buffer_present_probe;
extern volatile u8 unclamped_row_count_probe;
extern const u8 presentation_unclamped_line_ids_probe[9];
extern volatile u8 presentation_request_flags_probe;
extern volatile u8 source_is_banked_probe;
extern volatile i16 resource_xms_handle_probe;
extern volatile i16 resource_ems_handle_probe;
extern volatile u8 vm_sequence_active_probe;
extern volatile u16 vm_displayed_line_probe;
extern volatile u8 graphics_span_remap_table_probe[256];
extern volatile u8 scene_dispatch_blocked_probe;
extern volatile u8 ship_active_flags_low_probe;
extern volatile u8 bridge_redraw_pending_probe;
extern volatile u8 finale_requested_probe;
extern volatile u8 nav_choice_sound_gate_probe;
extern u16 list_entry_metric_probe;
extern volatile u16 list_read_wrap_index_probe;
extern volatile u16 palette_transition_percent_probe;
extern volatile u8 depth_opening_probe;
extern volatile u8 depth_step_probe;

i16 FAR pbm_image_load_and_decode_probe(
        volatile char FAR *path, volatile u8 FAR *file_buffer_end);
void FAR back_buffer_fill_probe(u8 color);
void NEAR resource_load_sequence_probe(u16 resource_id);
i16 FAR palette_blend_remap_table_build_probe(
        i16 negative_percent,
        u16 target_red,
        u16 target_green,
        u16 target_blue,
        volatile u8 FAR *table);
void NEAR ems_resource_flush_probe(u16 link_target_offset);
int NEAR list_d8c_state_le_one_probe(void);
void FAR blit_fill_row_probe(u8 color);

void FAR dlg_line_id_scene_dispatch_probe(u16 link_target_offset)
{
    volatile vm_record_triple_probe FAR *record;
    volatile char NEAR *image_path;
    u16 line;
    u16 index;

    resource_frame_presented_probe = 0u;
    line = vm_active_line_probe;
    if ((i16)line < 0) {
        return;
    }

    if ((vm_c2_presentation_gate_probe & 1u) == 0u) {
        if (line == 0x001du) {
            record = (volatile vm_record_triple_probe FAR *)MK_FP(
                    FP_SEG(vm_record_base_probe), vm_primary_c4_record_probe);
            alien_overlay_armed_probe =
                    record->related == vm_named_scruter_probe;
        } else if ((alien_overlay_armed_probe & 1u) != 0u) {
            temp_snd_trigger_probe = 1u;
            return;
        }

        if ((vm_scene_gate_probe & 1u) == 0u) {
            image_path = resource_index_probe[line].image_path;
            if (image_path == NO_SCENE_IMAGE) {
                loaded_scene_image_path_probe = NO_SCENE_IMAGE;
            } else if (image_path != loaded_scene_image_path_probe) {
                loaded_scene_image_path_probe = image_path;
                pbm_palette_refresh_probe = 1u;
                pbm_transparent_zero_probe = 1u;
                force_directory_probe = 1u;
                (void)pbm_image_load_and_decode_probe(
                        image_path, graphics_back_buffer_probe);
                force_directory_probe = 0u;
                pbm_palette_refresh_probe = 0u;
                pbm_transparent_zero_probe = 0u;
                memcpy(
                        presentation_palette_dwords_probe,
                        &scene_palette_dwords_probe[0x60],
                        sizeof(presentation_palette_dwords_probe));
            }

            if (loaded_scene_image_path_probe == NO_SCENE_IMAGE) {
                graphics_band_top_probe = resource_vertical_offset_probe;
                graphics_band_bottom_probe =
                        (u16)(resource_vertical_offset_probe + 0x0082u);
                back_buffer_fill_probe(0u);
                graphics_band_top_probe = 0u;
                graphics_band_bottom_probe = 200u;
            }
        }

        vm_c2_presentation_gate_probe = 1u;
        draw_via_back_buffer_probe = 0u;
        skip_back_buffer_present_probe = 0u;
        unclamped_row_count_probe = 0u;
        for (index = 0u; index < 8u; ++index) {
            if (presentation_unclamped_line_ids_probe[index] == (u8)line) {
                unclamped_row_count_probe = 1u;
                break;
            }
        }

        if (line == 2u || line == 7u) {
            draw_via_back_buffer_probe = 1u;
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
                presentation_request_flags_probe |= 2u;
                skip_back_buffer_present_probe = 1u;
                break;
            }
        }

        source_is_banked_probe = 0u;
        if (line == 8u
                && (u16)(resource_xms_handle_probe + resource_ems_handle_probe)
                        != 0xfffeu) {
            source_is_banked_probe = 1u;
        }

        resource_load_sequence_probe(line);
        if ((vm_sequence_active_probe | vm_scene_gate_probe) == 0u) {
            return;
        }
        line = vm_active_line_probe;
        if (line == vm_displayed_line_probe) {
            return;
        }
        vm_displayed_line_probe = line;
        (void)palette_blend_remap_table_build_probe(
                -50, 0u, 0u, 0u, graphics_span_remap_table_probe);
        return;
    }

    if ((scene_dispatch_blocked_probe & 1u) != 0u) {
        return;
    }
    ems_resource_flush_probe(link_target_offset);
    if (!list_d8c_state_le_one_probe()) {
        if ((ship_active_flags_low_probe & 8u) != 0u) {
            bridge_redraw_pending_probe = 1u;
        }
        if (vm_active_line_probe == 5u) {
            graphics_band_top_probe = 0x0023u;
            graphics_band_bottom_probe = 0x00a5u;
            blit_fill_row_probe(0u);
            graphics_band_top_probe = 0u;
            graphics_band_bottom_probe = 200u;
        }
        temp_snd_trigger_probe = (alien_overlay_armed_probe & 1u) != 0u;
        nav_choice_sound_gate_probe = (finale_requested_probe & 1u) != 0u;
        vm_c2_presentation_gate_probe = 0u;
        vm_displayed_line_probe = vm_active_line_probe;
        vm_active_line_probe = 0xffffu;
        presentation_request_flags_probe &= (u8)~2u;
        return;
    }

    if (vm_active_line_probe == 0x0027u) {
        if ((u16)(list_entry_metric_probe - list_read_wrap_index_probe)
                == 0x0014u) {
            palette_transition_percent_probe = 0u;
        }
    } else if ((ship_active_flags_low_probe & 8u) != 0u
            && (u16)(list_entry_metric_probe - list_read_wrap_index_probe)
                    == 8u) {
        depth_opening_probe = 1u;
        depth_step_probe = 6u;
    }
}
