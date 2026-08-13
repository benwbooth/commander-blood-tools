/* Codegen probe for the complete alien face-activation owner. */
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
#define XDB_CROOLIS_FREE_HEAD_OFFSET 0x0bd0u
#define XDB_CROOLIS_ACTIVE_LIST_HEAD_OFFSET 0x0c2cu
#define XDB_CROOLIS_ADVANCE_SECONDARY_OFFSET 0x2b2au
#define XDB_CROOLIS_ADVANCE_SWITCH_OFFSET 0x2b79u
#define XDB_CROOLIS_ADVANCE_REMOVE_OFFSET 0x2bbeu

typedef struct xdb_alien_face {
    xdb_u16 link;
    xdb_u16 vertex_0;
    xdb_u16 vertex_1;
    xdb_u16 vertex_2;
} xdb_alien_face;

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
    xdb_u16 texture_u;
    xdb_u16 texture_v;
    xdb_alien_projection_field_004 field_004;
    xdb_i16 object_y;
    xdb_i16 object_z;
    xdb_alien_screen_position screen;
    xdb_i32 depth;
    xdb_u16 clip_flags;
} xdb_alien_projection_vertex;

typedef union xdb_alien_texture_coordinate {
    xdb_u32 packed;
    struct {
        xdb_i16 u;
        xdb_i16 v;
    } component;
} xdb_alien_texture_coordinate;

typedef struct xdb_alien_raster_record {
    xdb_u16 next;
    xdb_u16 flags;
    xdb_u16 output_start;
    xdb_u16 output_end;
    xdb_i32 edge_0_position;
    xdb_i32 edge_0_step;
    xdb_u16 previous;
    xdb_u8 field_012[6];
    xdb_i32 edge_1_position;
    xdb_i32 edge_1_step;
    xdb_i32 depth_position;
    xdb_i32 depth_step;
    xdb_i32 depth_gradient;
    xdb_u16 advance_offset;
    xdb_i16 remaining;
    xdb_i16 secondary_remaining;
    xdb_i32 secondary_edge_position;
    xdb_i32 secondary_edge_step;
    xdb_i32 secondary_depth_position;
    xdb_i32 secondary_depth_step;
    xdb_i16 texture_u;
    xdb_i16 texture_v;
    xdb_i16 secondary_texture_u;
    xdb_i16 secondary_texture_v;
    xdb_i16 texture_u_step;
    xdb_i16 texture_v_step;
    xdb_i16 secondary_texture_u_step;
    xdb_i16 secondary_texture_v_step;
    xdb_i16 texture_du;
    xdb_i16 texture_dv;
    xdb_u16 texture_segment;
    xdb_u16 sort_next;
} xdb_alien_raster_record;

extern volatile xdb_u16 xdb_alien_palette_segment;
#define xdb_alien_texture_segment_base xdb_alien_palette_segment

void XDB_NEAR xdb_croolis_face_activate(
        const volatile xdb_alien_face XDB_FAR *face,
        xdb_u16 raster_segment);

#if defined(__WATCOMC__)
#pragma aux xdb_croolis_face_activate \
        parm [es si] [dx] modify exact [ax bx cx dx si di]
#endif

#define XDB_MULTIPLY_Q16(left, right) \
    ((xdb_i32)( \
            ((xdb_u32)( \
                    (xdb_i32)(xdb_i16)((xdb_u32)(left) >> 16) \
                    * (xdb_i32)(xdb_i16)((xdb_u32)(right) >> 16)) \
                    << 16) \
            + (xdb_u32)( \
                    (xdb_i32)(xdb_i16)((xdb_u32)(left) >> 16) \
                    * (xdb_i32)(xdb_u16)(right)) \
            + (xdb_u32)( \
                    (xdb_i32)(xdb_i16)((xdb_u32)(right) >> 16) \
                    * (xdb_i32)(xdb_u16)(left)) \
            + (((xdb_u32)(xdb_u16)(left) * (xdb_u32)(xdb_u16)(right)) \
                    >> 16)))
#define XDB_MULTIPLY_LOW(left, right) \
    ((xdb_i32)((xdb_u32)(left) * (xdb_u32)(right)))
#define XDB_ADD_I32(left, right) \
    ((xdb_i32)((xdb_u32)(left) + (xdb_u32)(right)))
#define XDB_SUB_I32(left, right) \
    ((xdb_i32)((xdb_u32)(left) - (xdb_u32)(right)))

void XDB_NEAR xdb_croolis_face_activate(
        const volatile xdb_alien_face XDB_FAR *face,
        xdb_u16 raster_segment)
{
    xdb_u16 geometry_segment = FP_SEG(face);
    const volatile xdb_i32 XDB_FAR *reciprocal_table = XDB_FAR_AT(
            const volatile xdb_i32,
            raster_segment,
            0u);
    volatile xdb_u16 XDB_FAR *free_head = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_CROOLIS_FREE_HEAD_OFFSET);
    const volatile xdb_alien_projection_vertex XDB_FAR *vertex_0;
    const volatile xdb_alien_projection_vertex XDB_FAR *vertex_1;
    const volatile xdb_alien_projection_vertex XDB_FAR *vertex_2;
    volatile xdb_alien_raster_record XDB_FAR *raster;
    xdb_alien_texture_coordinate texture_0;
    xdb_alien_texture_coordinate texture_1;
    xdb_alien_texture_coordinate texture_2;
    xdb_u32 screen_0;
    xdb_u32 screen_1;
    xdb_u32 screen_2;
    xdb_u16 x_0;
    xdb_u16 x_1;
    xdb_u16 x_2;
    xdb_u16 width_1;
    xdb_u16 width_2;
    xdb_i32 reciprocal_1;
    xdb_i32 reciprocal_2;
    xdb_i32 edge_1_step;
    xdb_i32 edge_2_step;
    xdb_i32 area;
    xdb_i32 denominator;
    xdb_i32 value_0;
    xdb_i32 delta_1;
    xdb_i32 delta_2;
    xdb_i16 word_step_1;
    xdb_i16 word_step_2;
    xdb_u16 clipped_columns;
    xdb_u16 clipping_mode = 0u;

    vertex_0 = XDB_FAR_AT(
            const volatile xdb_alien_projection_vertex,
            geometry_segment,
            face->vertex_0);
    vertex_1 = XDB_FAR_AT(
            const volatile xdb_alien_projection_vertex,
            geometry_segment,
            face->vertex_1);
    vertex_2 = XDB_FAR_AT(
            const volatile xdb_alien_projection_vertex,
            geometry_segment,
            face->vertex_2);
    if (*free_head == 0u) {
        return;
    }
    raster = XDB_FAR_AT(
            volatile xdb_alien_raster_record,
            raster_segment,
            *free_head);

    screen_0 = vertex_0->screen.packed;
    screen_1 = vertex_1->screen.packed;
    screen_2 = vertex_2->screen.packed;
    x_0 = (xdb_u16)screen_0;
    x_1 = (xdb_u16)screen_1;
    x_2 = (xdb_u16)screen_2;
    width_1 = (xdb_u16)(x_1 - x_0);
    width_2 = (xdb_u16)(x_2 - x_0);
    texture_0.packed = (xdb_u32)vertex_0->texture_u
            | ((xdb_u32)vertex_0->texture_v << 16);
    texture_1.packed = (xdb_u32)vertex_1->texture_u
            | ((xdb_u32)vertex_1->texture_v << 16);
    texture_2.packed = (xdb_u32)vertex_2->texture_u
            | ((xdb_u32)vertex_2->texture_v << 16);

    if (width_1 == 0u) {
        xdb_u16 vertical_span;

        if (width_2 == 0u || width_2 >= XDB_ALIEN_MAX_FACE_WIDTH) {
            return;
        }
        vertical_span = (xdb_u16)(
                (xdb_u16)(screen_1 >> 16) - (xdb_u16)(screen_0 >> 16));
        if ((xdb_i16)vertical_span <= 0
                || vertical_span >= XDB_ALIEN_MAX_FACE_WIDTH) {
            return;
        }

        reciprocal_1 = reciprocal_table[vertical_span];
        reciprocal_2 = reciprocal_table[width_2];
        raster->remaining = (xdb_i16)(width_2 - 1u);

        edge_2_step = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_2);
        raster->edge_0_step = edge_2_step;
        raster->edge_0_position = XDB_ADD_I32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_2_step >> 1);

        edge_1_step = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_1 >> 16)),
                reciprocal_2);
        raster->edge_1_step = edge_1_step;
        raster->edge_1_position = XDB_ADD_I32(
                (xdb_i32)(screen_1 & 0xffff0000ul), edge_1_step >> 1);

        delta_1 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_1.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_1);
        delta_2 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_2.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_2);
        raster->texture_du = (xdb_i16)(delta_1 >> 8);
        raster->texture_u_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_u = (xdb_i16)(
                ((xdb_u16)texture_0.packed << 8)
                + (xdb_i16)(delta_2 >> 9));

        delta_1 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_u16)(texture_1.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_1);
        delta_2 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_u16)(texture_2.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_2);
        raster->texture_dv = (xdb_i16)(delta_1 >> 8);
        raster->texture_v_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_v = (xdb_i16)(
                ((xdb_u16)(texture_0.packed >> 16) << 8)
                + (xdb_i16)(delta_2 >> 9));

        value_0 = XDB_MULTIPLY_Q16(
                XDB_SUB_I32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_step = value_0;
        raster->depth_position = XDB_ADD_I32(
                vertex_0->depth, value_0 >> 1);
        raster->depth_gradient = XDB_MULTIPLY_Q16(
                XDB_SUB_I32(vertex_1->depth, vertex_0->depth),
                reciprocal_1);
        raster->advance_offset = XDB_CROOLIS_ADVANCE_REMOVE_OFFSET;
    } else {
        if (width_2 == 0u) {
            return;
        }

        reciprocal_1 = reciprocal_table[width_1];
        reciprocal_2 = reciprocal_table[width_2];
        raster->remaining = (xdb_i16)(width_2 - 1u);
        edge_1_step = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_1 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_1);
        edge_2_step = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_2);
        area = XDB_SUB_I32(edge_2_step, edge_1_step);
        if (area >= 0) {
            return;
        }
        denominator = -(area >> 8);
        raster->edge_0_step = edge_2_step;
        raster->edge_1_step = edge_1_step;
        raster->edge_0_position = XDB_ADD_I32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_2_step >> 1);
        raster->edge_1_position = XDB_ADD_I32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_1_step >> 1);

        delta_1 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_1.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_1);
        delta_2 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_2.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_2);
        raster->texture_du = (xdb_i16)(
                XDB_SUB_I32(delta_1, delta_2) / denominator);
        raster->texture_u_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_u = (xdb_i16)(
                ((xdb_u16)texture_0.packed << 8)
                + (raster->texture_u_step >> 1));

        delta_1 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_u16)(texture_1.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_1);
        delta_2 = XDB_MULTIPLY_LOW(
                (xdb_i32)(xdb_u16)(texture_2.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_2);
        raster->texture_dv = (xdb_i16)(
                XDB_SUB_I32(delta_1, delta_2) / denominator);
        raster->texture_v_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_v = (xdb_i16)(
                ((xdb_u16)(texture_0.packed >> 16) << 8)
                + (raster->texture_v_step >> 1));

        value_0 = XDB_MULTIPLY_Q16(
                XDB_SUB_I32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_step = value_0;
        raster->depth_position = XDB_ADD_I32(
                vertex_0->depth, value_0 >> 1);
        delta_1 = XDB_MULTIPLY_LOW(
                XDB_SUB_I32(vertex_1->depth, vertex_0->depth),
                reciprocal_1);
        delta_2 = XDB_MULTIPLY_LOW(
                XDB_SUB_I32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_gradient = XDB_SUB_I32(delta_1, delta_2)
                / denominator;
        raster->depth_gradient >>= 8;

        if ((xdb_i16)(x_1 - x_2) > 0) {
            xdb_u16 secondary_width = (xdb_u16)(x_1 - x_2);
            xdb_i32 reciprocal = reciprocal_table[secondary_width];

            if ((xdb_i16)x_2 < 0) {
                clipped_columns = (xdb_u16)(0u - x_2);
                raster->remaining = (xdb_i16)(x_1 - 1u);

                delta_1 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)texture_1.packed
                                - (xdb_u16)texture_2.packed),
                        reciprocal);
                word_step_1 = (xdb_i16)(delta_1 >> 8);
                raster->texture_u_step = word_step_1;
                raster->texture_u = (xdb_i16)(
                        ((xdb_u16)texture_2.packed << 8)
                        + (xdb_i16)((xdb_u16)word_step_1 * clipped_columns));

                delta_1 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(texture_1.packed >> 16)
                                - (xdb_u16)(texture_2.packed >> 16)),
                        reciprocal);
                word_step_2 = (xdb_i16)(delta_1 >> 8);
                raster->texture_v_step = word_step_2;
                raster->texture_v = (xdb_i16)(
                        ((xdb_u16)(texture_2.packed >> 16) << 8)
                        + (xdb_i16)((xdb_u16)word_step_2 * clipped_columns));

                value_0 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_1 >> 16)
                                - (xdb_u16)(screen_2 >> 16)),
                        reciprocal);
                raster->edge_0_step = value_0;
                raster->edge_0_position = XDB_ADD_I32(
                        (xdb_i32)(screen_2 & 0xffff0000ul),
                        XDB_MULTIPLY_LOW(value_0, clipped_columns));

                value_0 = XDB_MULTIPLY_Q16(
                        XDB_SUB_I32(vertex_1->depth, vertex_2->depth),
                        reciprocal);
                raster->depth_step = value_0;
                raster->depth_position = XDB_ADD_I32(
                        vertex_2->depth,
                        XDB_MULTIPLY_LOW(value_0, clipped_columns));
                clipped_columns = (xdb_u16)(0u - x_0);
                raster->edge_1_position = XDB_ADD_I32(
                        raster->edge_1_position,
                        XDB_MULTIPLY_LOW(
                                raster->edge_1_step, clipped_columns));
                raster->advance_offset = XDB_CROOLIS_ADVANCE_REMOVE_OFFSET;
                clipping_mode = 1u;
            } else {
                raster->secondary_remaining = (xdb_i16)(secondary_width - 1u);

                delta_1 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)texture_1.packed
                                - (xdb_u16)texture_2.packed),
                        reciprocal);
                raster->secondary_texture_u_step = (xdb_i16)(delta_1 >> 8);
                raster->secondary_texture_u = (xdb_i16)(
                        ((xdb_u16)texture_2.packed << 8)
                        + (raster->secondary_texture_u_step >> 1));

                delta_1 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(texture_1.packed >> 16)
                                - (xdb_u16)(texture_2.packed >> 16)),
                        reciprocal);
                raster->secondary_texture_v_step = (xdb_i16)(delta_1 >> 8);
                raster->secondary_texture_v = (xdb_i16)(
                        ((xdb_u16)(texture_2.packed >> 16) << 8)
                        + (raster->secondary_texture_v_step >> 1));

                value_0 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_1 >> 16)
                                - (xdb_u16)(screen_2 >> 16)),
                        reciprocal);
                raster->secondary_edge_step = value_0;
                raster->secondary_edge_position = XDB_ADD_I32(
                        (xdb_i32)(screen_2 & 0xffff0000ul), value_0 >> 1);

                value_0 = XDB_MULTIPLY_Q16(
                        XDB_SUB_I32(vertex_1->depth, vertex_2->depth),
                        reciprocal);
                raster->secondary_depth_step = value_0;
                raster->secondary_depth_position = XDB_ADD_I32(
                        vertex_2->depth, value_0 >> 1);
                raster->advance_offset = XDB_CROOLIS_ADVANCE_SECONDARY_OFFSET;
            }
        } else if ((xdb_i16)(x_1 - x_2) < 0) {
            xdb_u16 secondary_width = (xdb_u16)(x_2 - x_1);
            xdb_i32 reciprocal = reciprocal_table[secondary_width];

            if ((xdb_i16)x_1 < 0) {
                clipped_columns = (xdb_u16)(0u - x_1);
                value_0 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_2 >> 16)
                                - (xdb_u16)(screen_1 >> 16)),
                        reciprocal);
                raster->edge_1_step = value_0;
                raster->edge_1_position = XDB_ADD_I32(
                        (xdb_i32)(screen_1 & 0xffff0000ul),
                        XDB_MULTIPLY_LOW(value_0, clipped_columns));
                raster->advance_offset = XDB_CROOLIS_ADVANCE_REMOVE_OFFSET;
                clipping_mode = 2u;
            } else {
                raster->remaining = (xdb_i16)(
                        (xdb_u16)raster->remaining - secondary_width);
                raster->secondary_remaining = (xdb_i16)(secondary_width - 1u);
                value_0 = XDB_MULTIPLY_LOW(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_2 >> 16)
                                - (xdb_u16)(screen_1 >> 16)),
                        reciprocal);
                raster->secondary_edge_step = value_0;
                raster->secondary_edge_position = XDB_ADD_I32(
                        (xdb_i32)(screen_1 & 0xffff0000ul), value_0 >> 1);
                raster->advance_offset = XDB_CROOLIS_ADVANCE_SWITCH_OFFSET;
            }
        } else {
            raster->advance_offset = XDB_CROOLIS_ADVANCE_REMOVE_OFFSET;
        }
    }

    raster->texture_segment = (xdb_u16)(
            xdb_alien_texture_segment_base
            + ((xdb_u16)(texture_0.packed >> 24) << 12));

    if ((xdb_i16)x_0 < 0 && clipping_mode != 1u) {
        clipped_columns = (xdb_u16)(0u - x_0);
        raster->remaining = (xdb_i16)(
                (xdb_u16)raster->remaining - clipped_columns);
        raster->edge_0_position = XDB_ADD_I32(
                raster->edge_0_position,
                XDB_MULTIPLY_LOW(raster->edge_0_step, clipped_columns));
        if (clipping_mode != 2u) {
            raster->edge_1_position = XDB_ADD_I32(
                    raster->edge_1_position,
                    XDB_MULTIPLY_LOW(
                            raster->edge_1_step, clipped_columns));
        }
        raster->depth_position = XDB_ADD_I32(
                raster->depth_position,
                XDB_MULTIPLY_LOW(raster->depth_step, clipped_columns));
        raster->texture_u = (xdb_i16)(
                (xdb_u16)raster->texture_u
                + (xdb_u16)raster->texture_u_step * clipped_columns);
        raster->texture_v = (xdb_i16)(
                (xdb_u16)raster->texture_v
                + (xdb_u16)raster->texture_v_step * clipped_columns);
    }

    *free_head = raster->next;
    {
        volatile xdb_alien_raster_record XDB_FAR *previous = XDB_FAR_AT(
                volatile xdb_alien_raster_record,
                raster_segment,
                XDB_CROOLIS_ACTIVE_LIST_HEAD_OFFSET);
        volatile xdb_alien_raster_record XDB_FAR *next = XDB_FAR_AT(
                volatile xdb_alien_raster_record,
                raster_segment,
                previous->next);

        if (raster->edge_0_position > next->edge_0_position
                || (raster->edge_0_position == next->edge_0_position
                && raster->edge_0_step > next->edge_0_step)) {
            do {
                previous = next;
                next = XDB_FAR_AT(
                        volatile xdb_alien_raster_record,
                        raster_segment,
                        next->next);
            } while (raster->edge_0_position > next->edge_0_position
                    || (raster->edge_0_position == next->edge_0_position
                    && raster->edge_0_step > next->edge_0_position));
        }
        previous->next = (xdb_u16)FP_OFF(raster);
        raster->previous = (xdb_u16)FP_OFF(previous);
        raster->next = (xdb_u16)FP_OFF(next);
        next->previous = (xdb_u16)FP_OFF(raster);
    }
}
