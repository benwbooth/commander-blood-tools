#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

void CB_NEAR presentation_line_zero_run(cb_u16 link_target_offset)
{
    blit_fill_row_5221(0u);
    back_buffer_fill(0u);
    vm_active_line = 0u;

    for (;;) {
        input_action_dispatch();
        if ((ship_3d_nav_choice_sound_gate & 1u) != 0u) {
            break;
        }

        dlg_line_id_scene_dispatch(link_target_offset);
        if ((vm_c2_presentation_gate & 1u) == 0u) {
            break;
        }

        chunky_to_planar_framebuffer(graphics_display_buffer);
        page_offset_helper();
        palette_upload_if_dirty();
    }

    ship_3d_nav_choice_sound_gate = 0u;
    vm_c2_presentation_gate = 0u;
    vm_active_line = 0xffffu;
}
