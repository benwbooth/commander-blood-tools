/* Codegen probe for the alien face-bucket prelude. */
#include <dos.h>

typedef unsigned char xdb_u8;
typedef unsigned int xdb_u16;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;
typedef signed long xdb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define XDB_FAR far
#define XDB_NEAR near
#define XDB_FAR_AT(type, segment, offset) \
    ((type XDB_FAR *)MK_FP((segment), (offset)))
#else
#define XDB_FAR
#define XDB_NEAR
#endif

#define XDB_ALIEN_MAX_FACE_WIDTH 0x01f4u
#define XDB_CROOLIS_FACE_BUCKETS_OFFSET 0x094eu
#define XDB_ALIEN_BEHIND_SCRATCH_OFFSET 0x07d4u

typedef union xdb_alien_projection_field_004 {
    xdb_i16 object_x;
    xdb_u16 projection_source_offset;
} xdb_alien_projection_field_004;

typedef union xdb_alien_screen_position {
    xdb_u32 packed;
    struct {
        xdb_i16 x;
        xdb_i16 y;
    } position;
} xdb_alien_screen_position;

typedef struct xdb_alien_projection_vertex {
    xdb_u8 field_000[0x04];
    xdb_alien_projection_field_004 field_004;
    xdb_i16 object_y;
    xdb_i16 object_z;
    xdb_alien_screen_position screen;
    xdb_i32 depth;
    xdb_u16 clip_flags;
} xdb_alien_projection_vertex;

typedef struct xdb_alien_face {
    xdb_u16 link;
    xdb_u16 vertex_0;
    xdb_u16 vertex_1;
    xdb_u16 vertex_2;
} xdb_alien_face;

typedef struct xdb_alien_projection_state {
    xdb_u8 unused;
} xdb_alien_projection_state;

typedef struct xdb_alien_projection_context {
    xdb_u8 field_000[0x16];
    volatile xdb_alien_projection_state XDB_NEAR *projection_root;
    xdb_u8 field_018[0x02];
    xdb_u16 state_count;
    xdb_u8 field_01c[0x06];
    xdb_u16 copy_offset;
    xdb_u8 field_024[0x02];
    xdb_u16 copy_count;
    xdb_u16 face_offset;
    xdb_u8 field_02a[0x02];
    xdb_u16 face_count;
} xdb_alien_projection_context;

extern volatile xdb_u16 xdb_alien_object_segment;
extern volatile xdb_u16 xdb_alien_raster_segment;
extern volatile xdb_u16 xdb_alien_control_latch;
extern volatile xdb_u16 xdb_alien_render_context_offsets[];
void XDB_NEAR xdb_alien_render_face_buckets_probe(void);
void XDB_NEAR xdb_alien_bucket_faces_then_render_probe(void);

#if defined(__WATCOMC__)
#pragma aux xdb_alien_bucket_faces_then_render_probe \
        modify exact [ax bx cx dx si di bp es]
#endif


void XDB_NEAR xdb_alien_bucket_faces_then_render_probe(void)
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

    xdb_alien_render_face_buckets_probe();
}
