#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_project_primary_mesh_then_render(void)
{
    volatile xdb_alien_primary_render_context XDB_NEAR *context =
            xdb_alien_primary_context_ptr;
    xdb_u16 geometry_segment = xdb_alien_object_segment;
    xdb_u16 vertex_offset = context->vertex_offset;
    xdb_u16 raster_segment;
    xdb_u16 face_count;
    xdb_u16 face_offset;

    xdb_alien_projection_common_clip = 0x800fu;
    xdb_alien_projection_field_2280 = 0u;
    xdb_alien_projection_remaining = context->vertex_count;
    do {
        volatile xdb_alien_projection_vertex XDB_FAR *vertex =
                XDB_FAR_AT(
                        volatile xdb_alien_projection_vertex,
                        geometry_segment,
                        vertex_offset);
        xdb_i32 object_x = vertex->field_004.object_x;
        xdb_i32 object_y = vertex->object_y;
        xdb_i32 object_z = vertex->object_z;
        xdb_u32 accumulator;
        xdb_i32 depth;

        vertex->clip_flags = 0x800fu;
        accumulator = (xdb_u32)xdb_alien_camera_matrix[6]
                * (xdb_u32)object_x;
        accumulator += (xdb_u32)xdb_alien_camera_matrix[7]
                * (xdb_u32)object_y;
        accumulator += (xdb_u32)xdb_alien_camera_matrix[8]
                * (xdb_u32)object_z;
        depth = (xdb_i32)accumulator;
        if (depth >= 0) {
            depth >>= 8;
            if (depth != 0) {
                xdb_u32 screen_x_accumulator =
                        (xdb_u32)xdb_alien_camera_matrix[0]
                        * (xdb_u32)object_x;
                xdb_u32 screen_y_accumulator =
                        (xdb_u32)xdb_alien_camera_matrix[3]
                        * (xdb_u32)object_x;
                xdb_i32 screen_x;
                xdb_i32 screen_y;
                xdb_u16 clip_flags = 0u;

                screen_x_accumulator +=
                        (xdb_u32)xdb_alien_camera_matrix[1]
                        * (xdb_u32)object_y;
                screen_x_accumulator +=
                        (xdb_u32)xdb_alien_camera_matrix[2]
                        * (xdb_u32)object_z;
                screen_y_accumulator +=
                        (xdb_u32)xdb_alien_camera_matrix[4]
                        * (xdb_u32)object_y;
                screen_y_accumulator +=
                        (xdb_u32)xdb_alien_camera_matrix[5]
                        * (xdb_u32)object_z;

                screen_x = (xdb_i32)screen_x_accumulator / depth;
                screen_y = (xdb_i32)screen_y_accumulator / depth;
                screen_y = (xdb_i32)(0UL - (xdb_u32)screen_y);
                screen_x = (xdb_i32)(
                        (xdb_u32)screen_x
                        + (xdb_u32)xdb_alien_screen_center_x);
                if (screen_x < 0) {
                    clip_flags = 0x0001u;
                }
                if (screen_x >= XDB_ALIEN_SCREEN_WIDTH) {
                    clip_flags = 0x0002u;
                }

                screen_y = (xdb_i32)(
                        (xdb_u32)screen_y
                        + (xdb_u32)xdb_alien_screen_center_y);
                if (screen_y < 0) {
                    clip_flags |= 0x0004u;
                }
                if (screen_y >= XDB_ALIEN_SCREEN_HEIGHT) {
                    clip_flags |= 0x0008u;
                }

                xdb_alien_projection_common_clip &= clip_flags;
                vertex->clip_flags = clip_flags;
                vertex->screen.position.x = (xdb_i16)screen_x;
                vertex->screen.position.y = (xdb_i16)screen_y;
            }
        }

        vertex_offset = (xdb_u16)(
                vertex_offset + sizeof(xdb_alien_projection_vertex));
    } while (--xdb_alien_projection_remaining != 0u);

    if (xdb_alien_projection_common_clip != 0u) {
        return;
    }

    context = xdb_alien_primary_context_ptr;
    raster_segment = xdb_alien_raster_segment;
    face_count = context->face_count;
    face_offset = context->face_offset;
    do {
        volatile xdb_alien_face XDB_FAR *face =
                XDB_FAR_AT(
                        volatile xdb_alien_face,
                        geometry_segment,
                        face_offset);
        xdb_u16 vertex_0_offset = face->vertex_0;
        xdb_u16 vertex_1_offset = face->vertex_1;
        xdb_u16 vertex_2_offset = face->vertex_2;
        volatile xdb_alien_projection_vertex XDB_FAR *vertex_0 =
                XDB_FAR_AT(
                        volatile xdb_alien_projection_vertex,
                        geometry_segment,
                        vertex_0_offset);
        volatile xdb_alien_projection_vertex XDB_FAR *vertex_1 =
                XDB_FAR_AT(
                        volatile xdb_alien_projection_vertex,
                        geometry_segment,
                        vertex_1_offset);
        volatile xdb_alien_projection_vertex XDB_FAR *vertex_2 =
                XDB_FAR_AT(
                        volatile xdb_alien_projection_vertex,
                        geometry_segment,
                        vertex_2_offset);
        xdb_u16 common_clip = vertex_0->clip_flags;

        common_clip &= vertex_1->clip_flags;
        common_clip &= vertex_2->clip_flags;
        if (common_clip == 0u) {
            xdb_i16 x_0 = vertex_0->screen.position.x;
            xdb_i16 x_1 = vertex_1->screen.position.x;
            xdb_i16 x_2 = vertex_2->screen.position.x;
            xdb_u16 span_1;
            xdb_u16 span_2;

            if (x_1 > x_2) {
                if (x_0 >= x_2) {
                    xdb_u16 saved_vertex = vertex_0_offset;
                    xdb_i16 saved_x = x_0;

                    vertex_0_offset = vertex_2_offset;
                    x_0 = x_2;
                    vertex_2_offset = vertex_1_offset;
                    x_2 = x_1;
                    vertex_1_offset = saved_vertex;
                    x_1 = saved_x;
                    face->vertex_0 = vertex_0_offset;
                    face->vertex_1 = vertex_1_offset;
                    face->vertex_2 = vertex_2_offset;
                }
            } else if (x_0 > x_1) {
                xdb_u16 saved_vertex = vertex_0_offset;
                xdb_i16 saved_x = x_0;

                vertex_0_offset = vertex_1_offset;
                x_0 = x_1;
                vertex_1_offset = vertex_2_offset;
                x_1 = x_2;
                vertex_2_offset = saved_vertex;
                x_2 = saved_x;
                face->vertex_0 = vertex_0_offset;
                face->vertex_1 = vertex_1_offset;
                face->vertex_2 = vertex_2_offset;
            }

            span_1 = (xdb_u16)((xdb_u16)x_1 - (xdb_u16)x_0);
            span_2 = (xdb_u16)((xdb_u16)x_2 - (xdb_u16)x_0);
            if (span_1 < XDB_ALIEN_MAX_FACE_WIDTH
                    && span_2 < XDB_ALIEN_MAX_FACE_WIDTH) {
                xdb_u16 doubled_x = (xdb_u16)((xdb_u16)x_0 << 1);
                xdb_u16 bucket_offset = XDB_SCRUT_FACE_BUCKETS_OFFSET;
                volatile xdb_u16 XDB_FAR *bucket;
                xdb_u16 previous_head;

                if ((xdb_i16)doubled_x >= 0) {
                    bucket_offset = (xdb_u16)(bucket_offset + doubled_x);
                }
                bucket = XDB_FAR_AT(
                        volatile xdb_u16,
                        raster_segment,
                        bucket_offset);
                previous_head = *bucket;
                *bucket = face_offset;
                face->link = previous_head;
            }
        }

        face_offset = (xdb_u16)(face_offset + sizeof(xdb_alien_face));
    } while (--face_count != 0u);

    xdb_scrut_render_face_buckets();
}
