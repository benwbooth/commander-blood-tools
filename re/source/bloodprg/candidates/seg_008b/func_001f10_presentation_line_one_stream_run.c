#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_hardware.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

void CB_NEAR presentation_line_one_stream_run(cb_u16 link_target_offset)
{
    ship_3d_nav_choice_sound_gate = 0u;
    ship_3d_plane_blit_crop_enabled = 0u;
    resource_vertical_offset = 0u;
    vm_active_line = 1u;

    snd_stream_source_load(snd_credits_voc_path);
    snd_stream_start();
    vga_dac_clear();
    blit_fill_row_5221(0u);
    back_buffer_fill(0u);

    for (;;) {
        input_action_dispatch();
        if ((ship_3d_nav_choice_sound_gate & 1u) != 0u) {
            return;
        }

        dlg_line_id_scene_dispatch(link_target_offset);
        if ((vm_c2_presentation_gate & 1u) == 0u) {
            return;
        }

        snd_stream_refill();
        chunky_to_planar_framebuffer(graphics_display_buffer);
        page_offset_helper();
        palette_upload_if_dirty();
    }
}
