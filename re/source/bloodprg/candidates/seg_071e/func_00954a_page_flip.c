#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

cb_u16 CB_FAR page_flip(void)
{
    bloodprg_graphics_buffer_ptr volatile saved_display_buffer;
    volatile cb_u16 first_object_id;
    volatile cb_u16 last_object_id;
    cb_u16 frame;

    palette_dirty = 1;
    saved_display_buffer = bloodprg_display_buffer;
    bloodprg_display_buffer = bloodprg_secondary_buffer;

    blit_fill_row_5221(0);
    ship_3d_projection_matrix_build();
    ship_3d_point_cloud_project();
    ship_3d_object_sprite_project();
    first_object_id = 0x0015u;
    last_object_id = 0x001fu;
    sprite_slot_commit_dirty_range(first_object_id, last_object_id);
    sprite_slot_dirty_range_render(first_object_id, last_object_id);

    bloodprg_display_buffer = saved_display_buffer;
    if ((vm_ship_active_flags & 1u) != 0) {
        return first_object_id;
    }

    page_flip_transparent_zero = 1;
    bloodprg_dirty_copy_flags = 1;
    frame = (cb_u16)vm_bridge_view_frame;
    bridge_panorama_frame_load(frame);
    return frame;
}
