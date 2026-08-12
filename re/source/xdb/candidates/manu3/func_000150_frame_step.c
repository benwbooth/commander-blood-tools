#include "../include/xdb_manu3.h"

void XDB_FAR xdb_manu3_frame_step(void)
{
    if (xdb_manu3_data_segment == 0u) {
        return;
    }

    xdb_manu3_framebuffer_segment = (xdb_u16)(
            0xa000u + (xdb_manu3_framebuffer_window_offset >> 4));
    xdb_manu3_tween_step();
    xdb_manu3_matrix_build();
    xdb_manu3_entity_project();
    xdb_manu3_face_builder_next();
}
