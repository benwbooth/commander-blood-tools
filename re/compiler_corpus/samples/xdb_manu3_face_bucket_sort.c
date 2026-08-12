/* Codegen probe for the MANU3 projected-face bucket sorter. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define FAR_AT(type, segment, offset) \
    ((type FAR *)MK_FP((segment), (offset)))
#define BUCKET_HEADS_OFFSET 0x0686u
#define MAX_FACE_HEIGHT 0x0190u

typedef struct face_record {
    u16 link;
    u16 vertex_0;
    u16 vertex_1;
    u16 vertex_2;
} face_record;

typedef struct projected_vertex {
    u8 field_000[0x0A];
    i16 screen_y;
    u8 field_00c[0x06];
    u16 clip_flags;
} projected_vertex;

extern volatile u16 face_list_offset;
extern volatile u16 face_count;
extern void NEAR span_renderer_init_probe(void);

void NEAR xdb_manu3_face_bucket_sort_probe(
        u16 geometry_segment,
        u16 raster_segment);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_face_bucket_sort_probe \
        parm [ax] [dx] modify exact [ax bx cx dx si di bp]
#pragma aux span_renderer_init_probe \
        modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_face_bucket_sort_probe(
        u16 geometry_segment,
        u16 raster_segment)
{
    u16 count = face_count;
    u16 face_offset = face_list_offset;

    do {
        volatile face_record FAR *face =
                FAR_AT(volatile face_record, geometry_segment, face_offset);
        u16 vertex_0_offset = face->vertex_0;
        u16 vertex_1_offset = face->vertex_1;
        u16 vertex_2_offset = face->vertex_2;
        volatile projected_vertex FAR *vertex_0 =
                FAR_AT(
                        volatile projected_vertex,
                        geometry_segment,
                        vertex_0_offset);
        volatile projected_vertex FAR *vertex_1 =
                FAR_AT(
                        volatile projected_vertex,
                        geometry_segment,
                        vertex_1_offset);
        volatile projected_vertex FAR *vertex_2 =
                FAR_AT(
                        volatile projected_vertex,
                        geometry_segment,
                        vertex_2_offset);
        u16 common_clip = vertex_0->clip_flags;

        common_clip &= vertex_1->clip_flags;
        common_clip &= vertex_2->clip_flags;
        if (common_clip == 0u) {
            i16 y_0 = vertex_0->screen_y;
            i16 y_1 = vertex_1->screen_y;
            i16 y_2 = vertex_2->screen_y;
            u16 span_1;
            u16 span_2;

            if (y_1 > y_2) {
                if (y_0 >= y_2) {
                    u16 saved_vertex = vertex_0_offset;
                    i16 saved_y = y_0;

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
                u16 saved_vertex = vertex_0_offset;
                i16 saved_y = y_0;

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

            span_1 = (u16)((u16)y_1 - (u16)y_0);
            span_2 = (u16)((u16)y_2 - (u16)y_0);
            if (span_1 < MAX_FACE_HEIGHT && span_2 < MAX_FACE_HEIGHT) {
                u16 doubled_y = (u16)((u16)y_0 << 1);
                u16 bucket_offset = BUCKET_HEADS_OFFSET;
                u16 previous_head;
                volatile u16 FAR *bucket;

                if ((i16)doubled_y >= 0) {
                    bucket_offset = (u16)(bucket_offset + doubled_y);
                }
                bucket = FAR_AT(
                        volatile u16,
                        raster_segment,
                        bucket_offset);
                previous_head = *bucket;
                *bucket = face_offset;
                face->link = previous_head;
            }
        }

        face_offset = (u16)(face_offset + sizeof(face_record));
    } while (--count != 0u);

    span_renderer_init_probe();
}
