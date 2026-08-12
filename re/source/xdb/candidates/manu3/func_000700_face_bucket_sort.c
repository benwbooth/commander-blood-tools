#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_face_bucket_sort(
        xdb_u16 geometry_segment,
        xdb_u16 raster_segment)
{
    xdb_u16 count = xdb_manu3_face_count;
    xdb_u16 face_offset = xdb_manu3_face_list_offset;

    do {
        volatile xdb_manu3_face XDB_FAR *face =
                XDB_FAR_AT(
                        volatile xdb_manu3_face,
                        geometry_segment,
                        face_offset);
        xdb_u16 vertex_0_offset = face->vertex_0;
        xdb_u16 vertex_1_offset = face->vertex_1;
        xdb_u16 vertex_2_offset = face->vertex_2;
        volatile xdb_manu3_vertex XDB_FAR *vertex_0 =
                XDB_FAR_AT(
                        volatile xdb_manu3_vertex,
                        geometry_segment,
                        vertex_0_offset);
        volatile xdb_manu3_vertex XDB_FAR *vertex_1 =
                XDB_FAR_AT(
                        volatile xdb_manu3_vertex,
                        geometry_segment,
                        vertex_1_offset);
        volatile xdb_manu3_vertex XDB_FAR *vertex_2 =
                XDB_FAR_AT(
                        volatile xdb_manu3_vertex,
                        geometry_segment,
                        vertex_2_offset);
        xdb_u16 common_clip = vertex_0->clip_flags;

        common_clip &= vertex_1->clip_flags;
        common_clip &= vertex_2->clip_flags;
        if (common_clip == 0u) {
            xdb_i16 y_0 = vertex_0->screen_y;
            xdb_i16 y_1 = vertex_1->screen_y;
            xdb_i16 y_2 = vertex_2->screen_y;
            xdb_u16 span_1;
            xdb_u16 span_2;

            if (y_1 > y_2) {
                if (y_0 >= y_2) {
                    xdb_u16 saved_vertex = vertex_0_offset;
                    xdb_i16 saved_y = y_0;

                    vertex_0_offset = vertex_2_offset;
                    y_0 = y_2;
                    vertex_2_offset = vertex_1_offset;
                    y_2 = y_1;
                    vertex_1_offset = saved_vertex;
                    y_1 = saved_y;
                    face->vertex_0 = vertex_0_offset;
                    face->vertex_1 = vertex_1_offset;
                    face->vertex_2 = vertex_2_offset;
                }
            } else if (y_0 > y_1) {
                xdb_u16 saved_vertex = vertex_0_offset;
                xdb_i16 saved_y = y_0;

                vertex_0_offset = vertex_1_offset;
                y_0 = y_1;
                vertex_1_offset = vertex_2_offset;
                y_1 = y_2;
                vertex_2_offset = saved_vertex;
                y_2 = saved_y;
                face->vertex_0 = vertex_0_offset;
                face->vertex_1 = vertex_1_offset;
                face->vertex_2 = vertex_2_offset;
            }

            span_1 = (xdb_u16)((xdb_u16)y_1 - (xdb_u16)y_0);
            span_2 = (xdb_u16)((xdb_u16)y_2 - (xdb_u16)y_0);
            if (span_1 < XDB_MANU3_MAX_FACE_HEIGHT
                    && span_2 < XDB_MANU3_MAX_FACE_HEIGHT) {
                xdb_u16 doubled_y = (xdb_u16)((xdb_u16)y_0 << 1);
                xdb_u16 bucket_offset = XDB_MANU3_BUCKET_HEADS_OFFSET;
                xdb_u16 previous_head;
                volatile xdb_u16 XDB_FAR *bucket;

                if ((xdb_i16)doubled_y >= 0) {
                    bucket_offset = (xdb_u16)(bucket_offset + doubled_y);
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

        face_offset = (xdb_u16)(face_offset + sizeof(xdb_manu3_face));
    } while (--count != 0u);

    xdb_manu3_span_renderer_init();
}
