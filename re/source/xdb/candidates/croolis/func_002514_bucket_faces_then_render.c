#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_bucket_faces_then_render(void)
{
    xdb_u16 geometry_segment = xdb_alien_object_segment;
    xdb_u16 raster_segment = xdb_alien_raster_segment;
    volatile xdb_u16 XDB_FAR *behind_scratch =
            XDB_FAR_AT(
                    volatile xdb_u16,
                    raster_segment,
                    XDB_ALIEN_BEHIND_SCRATCH_OFFSET);
    xdb_u16 context_index = 0u;

    *behind_scratch = 0u;
    do {
        xdb_u16 context_offset =
                xdb_alien_render_context_offsets[context_index++];
        volatile xdb_alien_projection_context XDB_NEAR *context =
                (volatile xdb_alien_projection_context XDB_NEAR *)context_offset;
        xdb_u16 face_count = context->face_count;
        xdb_u16 face_offset = context->face_offset;

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
                xdb_u16 combined_clip = vertex_0->clip_flags;
                xdb_i16 x_0 = vertex_0->screen.position.x;
                xdb_i16 x_1 = vertex_1->screen.position.x;
                xdb_i16 x_2 = vertex_2->screen.position.x;
                xdb_u16 span_1;
                xdb_u16 span_2;

                combined_clip |= vertex_1->clip_flags;
                combined_clip |= vertex_2->clip_flags;
                if ((xdb_i16)combined_clip < 0) {
                    *behind_scratch = 1u;
                }

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
                    xdb_u16 bucket_offset = XDB_CROOLIS_FACE_BUCKETS_OFFSET;
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

        if (*behind_scratch != 0u) {
            *behind_scratch = 0u;
            xdb_alien_control_latch = context_offset;
        }
    } while (xdb_alien_render_context_offsets[context_index] != 0u);

    xdb_croolis_render_face_buckets();
}
