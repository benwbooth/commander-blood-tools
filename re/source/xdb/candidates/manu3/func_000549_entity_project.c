#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_entity_project(void)
{
    xdb_u16 geometry_segment = xdb_manu3_segments.work_segment_0;
    xdb_u16 state_count = xdb_manu3_projection_state_count;
    xdb_u16 copy_count;
    volatile xdb_manu3_projection_state XDB_NEAR *state =
            xdb_manu3_projection_states;

    do {
        xdb_u16 vertex_offset = state->vertex_offset;

        xdb_manu3_projection_remaining = state->vertex_count;
        xdb_manu3_projection_field_224e = 0;
        do {
            volatile xdb_manu3_vertex XDB_FAR *vertex =
                    XDB_FAR_AT(
                            volatile xdb_manu3_vertex,
                            geometry_segment,
                            vertex_offset);
            xdb_i32 object_x = vertex->field_004.object_x;
            xdb_i32 object_y = vertex->object_y;
            xdb_i32 object_z = vertex->object_z;
            xdb_u32 accumulator;
            xdb_i32 depth;

            vertex->clip_flags = 0x8000u;
            accumulator = (xdb_u32)state->matrix[2][0] * (xdb_u32)object_x;
            accumulator += (xdb_u32)state->matrix[2][1] * (xdb_u32)object_y;
            accumulator += (xdb_u32)state->matrix[2][2] * (xdb_u32)object_z;
            accumulator += (xdb_u32)state->translation[2];
            depth = (xdb_i32)accumulator >> 8;
            vertex->depth = depth;

            if (depth > 0) {
                xdb_u32 screen_x_accumulator;
                xdb_u32 screen_y_accumulator;
                xdb_i32 screen_x;
                xdb_i32 screen_y;
                xdb_u16 clip_flags = 0;

                screen_y_accumulator = (xdb_u32)state->matrix[1][0]
                        * (xdb_u32)object_x;
                screen_y_accumulator += (xdb_u32)state->matrix[1][1]
                        * (xdb_u32)object_y;
                screen_y_accumulator += (xdb_u32)state->matrix[1][2]
                        * (xdb_u32)object_z;
                screen_y_accumulator += (xdb_u32)state->translation[1];
                screen_x_accumulator = (xdb_u32)state->matrix[0][0]
                        * (xdb_u32)object_x;
                screen_x_accumulator += (xdb_u32)state->matrix[0][1]
                        * (xdb_u32)object_y;
                screen_x_accumulator += (xdb_u32)state->matrix[0][2]
                        * (xdb_u32)object_z;
                screen_x_accumulator += (xdb_u32)state->translation[0];

                screen_x = (xdb_i32)screen_x_accumulator / depth;
                screen_y = (xdb_i32)(
                        0u - (xdb_u32)((xdb_i32)screen_y_accumulator / depth));
                screen_x = (xdb_i32)(
                        (xdb_u32)screen_x
                        + (xdb_u32)xdb_manu3_screen_center_x);
                if (screen_x < 0) {
                    clip_flags = 0x0001u;
                    if (screen_x <= -40) {
                        screen_x = -39;
                    }
                }
                if (screen_x >= 320) {
                    clip_flags = 0x0002u;
                    if (screen_x >= 360) {
                        screen_x = 359;
                    }
                }

                screen_y = (xdb_i32)(
                        (xdb_u32)screen_y
                        + (xdb_u32)xdb_manu3_screen_center_y);
                if (screen_y < 0) {
                    clip_flags |= 0x0004u;
                    if (screen_y <= -100) {
                        screen_y = -99;
                    }
                }
                if (screen_y >= 200) {
                    clip_flags |= 0x0008u;
                    if (screen_y >= 300) {
                        screen_y = 299;
                    }
                }

                vertex->clip_flags = clip_flags;
                vertex->screen.position.x = (xdb_i16)screen_x;
                vertex->screen.position.y = (xdb_i16)screen_y;
            }

            vertex_offset = (xdb_u16)(
                    vertex_offset + sizeof(xdb_manu3_vertex));
        } while (--xdb_manu3_projection_remaining != 0u);

        ++state;
    } while (--state_count != 0u);

    copy_count = xdb_manu3_projection_copy_count;
    if (copy_count != 0u) {
        xdb_u16 destination_offset = xdb_manu3_projection_copy_offset;

        do {
            volatile xdb_manu3_vertex XDB_FAR *destination =
                    XDB_FAR_AT(
                            volatile xdb_manu3_vertex,
                            geometry_segment,
                            destination_offset);
            volatile xdb_manu3_vertex XDB_FAR *source =
                    XDB_FAR_AT(
                            volatile xdb_manu3_vertex,
                            geometry_segment,
                            destination->field_004.projection_source_offset);
            xdb_u32 screen = source->screen.packed;
            xdb_i32 depth = source->depth;
            xdb_u16 clip_flags = source->clip_flags;

            destination->screen.packed = screen;
            destination->depth = depth;
            destination->clip_flags = clip_flags;
            destination_offset = (xdb_u16)(
                    destination_offset + sizeof(xdb_manu3_vertex));
        } while (--copy_count != 0u);
    }
}
