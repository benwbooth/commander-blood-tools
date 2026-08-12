#include "../include/xdb_manu3.h"

static xdb_i32 xdb_manu3_multiply_q16(xdb_i32 left, xdb_i32 right)
{
    xdb_i16 left_high = (xdb_i16)((xdb_u32)left >> 16);
    xdb_u16 left_low = (xdb_u16)left;
    xdb_i16 right_high = (xdb_i16)((xdb_u32)right >> 16);
    xdb_u16 right_low = (xdb_u16)right;
    xdb_u32 result;

    result = (xdb_u32)((xdb_i32)left_high * (xdb_i32)right_high) << 16;
    result += (xdb_u32)((xdb_i32)left_high * (xdb_i32)right_low);
    result += (xdb_u32)((xdb_i32)right_high * (xdb_i32)left_low);
    result += ((xdb_u32)left_low * (xdb_u32)right_low) >> 16;
    return (xdb_i32)result;
}

static xdb_i32 xdb_manu3_multiply_low(xdb_i32 left, xdb_i32 right)
{
    return (xdb_i32)((xdb_u32)left * (xdb_u32)right);
}

static xdb_i32 xdb_manu3_add_i32(xdb_i32 left, xdb_i32 right)
{
    return (xdb_i32)((xdb_u32)left + (xdb_u32)right);
}

static xdb_i32 xdb_manu3_sub_i32(xdb_i32 left, xdb_i32 right)
{
    return (xdb_i32)((xdb_u32)left - (xdb_u32)right);
}

void XDB_NEAR xdb_manu3_face_activate(
        const volatile xdb_manu3_face XDB_FAR *face)
{
    xdb_u16 geometry_segment = FP_SEG(face);
    const volatile xdb_manu3_vertex XDB_FAR *vertex_0;
    const volatile xdb_manu3_vertex XDB_FAR *vertex_1;
    const volatile xdb_manu3_vertex XDB_FAR *vertex_2;
    volatile xdb_manu3_raster_record XDB_NEAR *raster;
    xdb_manu3_texture_coordinate texture_0;
    xdb_manu3_texture_coordinate texture_1;
    xdb_manu3_texture_coordinate texture_2;
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
            const volatile xdb_manu3_vertex,
            geometry_segment,
            face->vertex_0);
    vertex_1 = XDB_FAR_AT(
            const volatile xdb_manu3_vertex,
            geometry_segment,
            face->vertex_1);
    vertex_2 = XDB_FAR_AT(
            const volatile xdb_manu3_vertex,
            geometry_segment,
            face->vertex_2);
    raster = (volatile xdb_manu3_raster_record XDB_NEAR *)
            xdb_manu3_active_raster_offset;
    if (raster == 0) {
        return;
    }

    screen_0 = vertex_0->screen.packed;
    screen_1 = vertex_1->screen.packed;
    screen_2 = vertex_2->screen.packed;
    x_0 = (xdb_u16)screen_0;
    x_1 = (xdb_u16)screen_1;
    x_2 = (xdb_u16)screen_2;
    width_1 = (xdb_u16)(x_1 - x_0);
    width_2 = (xdb_u16)(x_2 - x_0);
    texture_0.packed = (xdb_u32)vertex_0->link
            | ((xdb_u32)vertex_0->field_002 << 16);
    texture_1.packed = (xdb_u32)vertex_1->link
            | ((xdb_u32)vertex_1->field_002 << 16);
    texture_2.packed = (xdb_u32)vertex_2->link
            | ((xdb_u32)vertex_2->field_002 << 16);

    if (width_1 == 0u) {
        xdb_u16 vertical_span;

        if (width_2 == 0u || width_2 >= XDB_MANU3_MAX_FACE_WIDTH) {
            return;
        }
        vertical_span = (xdb_u16)(
                (xdb_u16)(screen_1 >> 16) - (xdb_u16)(screen_0 >> 16));
        if ((xdb_i16)vertical_span <= 0
                || vertical_span >= XDB_MANU3_MAX_FACE_WIDTH) {
            return;
        }

        reciprocal_1 = xdb_manu3_reciprocal_table[vertical_span];
        reciprocal_2 = xdb_manu3_reciprocal_table[width_2];
        raster->remaining = (xdb_i16)(width_2 - 1u);

        edge_2_step = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_2);
        raster->edge_0_step = edge_2_step;
        raster->edge_0_position = xdb_manu3_add_i32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_2_step >> 1);

        edge_1_step = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_1 >> 16)),
                reciprocal_2);
        raster->edge_1_step = edge_1_step;
        raster->edge_1_position = xdb_manu3_add_i32(
                (xdb_i32)(screen_1 & 0xffff0000ul), edge_1_step >> 1);

        delta_1 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_1.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_1);
        delta_2 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_2.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_2);
        raster->texture_du = (xdb_i16)(delta_1 >> 8);
        raster->texture_u_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_u = (xdb_i16)(
                ((xdb_u16)texture_0.packed << 8)
                + (raster->texture_u_step >> 1));

        delta_1 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_u16)(texture_1.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_1);
        delta_2 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_u16)(texture_2.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_2);
        raster->texture_dv = (xdb_i16)(delta_1 >> 8);
        raster->texture_v_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_v = (xdb_i16)(
                ((xdb_u16)(texture_0.packed >> 16) << 8)
                + (raster->texture_v_step >> 1));

        value_0 = xdb_manu3_multiply_q16(
                xdb_manu3_sub_i32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_step = value_0;
        raster->depth_position = xdb_manu3_add_i32(
                vertex_0->depth, value_0 >> 1);
        raster->depth_gradient = xdb_manu3_multiply_q16(
                xdb_manu3_sub_i32(vertex_1->depth, vertex_0->depth),
                reciprocal_1);
        raster->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
    } else {
        if (width_2 == 0u) {
            return;
        }

        reciprocal_1 = xdb_manu3_reciprocal_table[width_1];
        reciprocal_2 = xdb_manu3_reciprocal_table[width_2];
        raster->remaining = (xdb_i16)(width_2 - 1u);
        edge_1_step = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_1 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_1);
        edge_2_step = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)(screen_2 >> 16)
                        - (xdb_u16)(screen_0 >> 16)),
                reciprocal_2);
        area = xdb_manu3_sub_i32(edge_2_step, edge_1_step);
        if (area >= 0) {
            return;
        }
        denominator = -(area >> 8);
        raster->edge_0_step = edge_2_step;
        raster->edge_1_step = edge_1_step;
        raster->edge_0_position = xdb_manu3_add_i32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_2_step >> 1);
        raster->edge_1_position = xdb_manu3_add_i32(
                (xdb_i32)(screen_0 & 0xffff0000ul), edge_1_step >> 1);

        delta_1 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_1.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_1);
        delta_2 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_i16)(
                        (xdb_u16)texture_2.packed
                        - (xdb_u16)texture_0.packed),
                reciprocal_2);
        raster->texture_du = (xdb_i16)(
                xdb_manu3_sub_i32(delta_1, delta_2) / denominator);
        raster->texture_u_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_u = (xdb_i16)(
                ((xdb_u16)texture_0.packed << 8)
                + (raster->texture_u_step >> 1));

        delta_1 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_u16)(texture_1.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_1);
        delta_2 = xdb_manu3_multiply_low(
                (xdb_i32)(xdb_u16)(texture_2.packed >> 16)
                - (xdb_i32)(xdb_u16)(texture_0.packed >> 16),
                reciprocal_2);
        raster->texture_dv = (xdb_i16)(
                xdb_manu3_sub_i32(delta_1, delta_2) / denominator);
        raster->texture_v_step = (xdb_i16)(delta_2 >> 8);
        raster->texture_v = (xdb_i16)(
                ((xdb_u16)(texture_0.packed >> 16) << 8)
                + (raster->texture_v_step >> 1));

        value_0 = xdb_manu3_multiply_q16(
                xdb_manu3_sub_i32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_step = value_0;
        raster->depth_position = xdb_manu3_add_i32(
                vertex_0->depth, value_0 >> 1);
        delta_1 = xdb_manu3_multiply_low(
                xdb_manu3_sub_i32(vertex_1->depth, vertex_0->depth),
                reciprocal_1);
        delta_2 = xdb_manu3_multiply_low(
                xdb_manu3_sub_i32(vertex_2->depth, vertex_0->depth),
                reciprocal_2);
        raster->depth_gradient = xdb_manu3_sub_i32(delta_1, delta_2)
                / denominator;
        raster->depth_gradient >>= 8;

        if ((xdb_i16)(x_1 - x_2) > 0) {
            xdb_u16 secondary_width = (xdb_u16)(x_1 - x_2);
            xdb_i32 reciprocal =
                    xdb_manu3_reciprocal_table[secondary_width];

            if ((xdb_i16)x_2 < 0) {
                clipped_columns = (xdb_u16)(0u - x_2);
                raster->remaining = (xdb_i16)(x_1 - 1u);

                delta_1 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)texture_1.packed
                                - (xdb_u16)texture_2.packed),
                        reciprocal);
                word_step_1 = (xdb_i16)(delta_1 >> 8);
                raster->texture_u_step = word_step_1;
                raster->texture_u = (xdb_i16)(
                        ((xdb_u16)texture_2.packed << 8)
                        + (xdb_i16)((xdb_u16)word_step_1 * clipped_columns));

                delta_1 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(texture_1.packed >> 16)
                                - (xdb_u16)(texture_2.packed >> 16)),
                        reciprocal);
                word_step_2 = (xdb_i16)(delta_1 >> 8);
                raster->texture_v_step = word_step_2;
                raster->texture_v = (xdb_i16)(
                        ((xdb_u16)(texture_2.packed >> 16) << 8)
                        + (xdb_i16)((xdb_u16)word_step_2 * clipped_columns));

                value_0 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_1 >> 16)
                                - (xdb_u16)(screen_2 >> 16)),
                        reciprocal);
                raster->edge_0_step = value_0;
                raster->edge_0_position = xdb_manu3_add_i32(
                        (xdb_i32)(screen_2 & 0xffff0000ul),
                        xdb_manu3_multiply_low(value_0, clipped_columns));

                value_0 = xdb_manu3_multiply_q16(
                        xdb_manu3_sub_i32(vertex_1->depth, vertex_2->depth),
                        reciprocal);
                raster->depth_step = value_0;
                raster->depth_position = xdb_manu3_add_i32(
                        vertex_2->depth,
                        xdb_manu3_multiply_low(value_0, clipped_columns));
                clipped_columns = (xdb_u16)(0u - x_0);
                raster->edge_1_position = xdb_manu3_add_i32(
                        raster->edge_1_position,
                        xdb_manu3_multiply_low(
                                raster->edge_1_step, clipped_columns));
                raster->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
                clipping_mode = 1u;
            } else {
                raster->secondary_remaining = (xdb_i16)(secondary_width - 1u);

                delta_1 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)texture_1.packed
                                - (xdb_u16)texture_2.packed),
                        reciprocal);
                raster->secondary_texture_u_step = (xdb_i16)(delta_1 >> 8);
                raster->secondary_texture_u = (xdb_i16)(
                        ((xdb_u16)texture_2.packed << 8)
                        + (raster->secondary_texture_u_step >> 1));

                delta_1 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(texture_1.packed >> 16)
                                - (xdb_u16)(texture_2.packed >> 16)),
                        reciprocal);
                raster->secondary_texture_v_step = (xdb_i16)(delta_1 >> 8);
                raster->secondary_texture_v = (xdb_i16)(
                        ((xdb_u16)(texture_2.packed >> 16) << 8)
                        + (raster->secondary_texture_v_step >> 1));

                value_0 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_1 >> 16)
                                - (xdb_u16)(screen_2 >> 16)),
                        reciprocal);
                raster->secondary_edge_step = value_0;
                raster->secondary_edge_position = xdb_manu3_add_i32(
                        (xdb_i32)(screen_2 & 0xffff0000ul), value_0 >> 1);

                value_0 = xdb_manu3_multiply_q16(
                        xdb_manu3_sub_i32(vertex_1->depth, vertex_2->depth),
                        reciprocal);
                raster->secondary_depth_step = value_0;
                raster->secondary_depth_position = xdb_manu3_add_i32(
                        vertex_2->depth, value_0 >> 1);
                raster->advance_offset = XDB_MANU3_ADVANCE_SECONDARY_OFFSET;
            }
        } else if ((xdb_i16)(x_1 - x_2) < 0) {
            xdb_u16 secondary_width = (xdb_u16)(x_2 - x_1);
            xdb_i32 reciprocal =
                    xdb_manu3_reciprocal_table[secondary_width];

            if ((xdb_i16)x_1 < 0) {
                clipped_columns = (xdb_u16)(0u - x_1);
                value_0 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_2 >> 16)
                                - (xdb_u16)(screen_1 >> 16)),
                        reciprocal);
                raster->edge_1_step = value_0;
                raster->edge_1_position = xdb_manu3_add_i32(
                        (xdb_i32)(screen_1 & 0xffff0000ul),
                        xdb_manu3_multiply_low(value_0, clipped_columns));
                raster->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
                clipping_mode = 2u;
            } else {
                raster->remaining = (xdb_i16)(
                        (xdb_u16)raster->remaining - secondary_width);
                raster->secondary_remaining = (xdb_i16)(secondary_width - 1u);
                value_0 = xdb_manu3_multiply_low(
                        (xdb_i32)(xdb_i16)(
                                (xdb_u16)(screen_2 >> 16)
                                - (xdb_u16)(screen_1 >> 16)),
                        reciprocal);
                raster->secondary_edge_step = value_0;
                raster->secondary_edge_position = xdb_manu3_add_i32(
                        (xdb_i32)(screen_1 & 0xffff0000ul), value_0 >> 1);
                raster->advance_offset = XDB_MANU3_ADVANCE_SWITCH_OFFSET;
            }
        } else {
            raster->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
        }
    }

    raster->texture_segment = (xdb_u16)(
            xdb_manu3_segments.work_segment_1
            + ((xdb_u16)(texture_0.packed >> 24) << 12));

    if ((xdb_i16)x_0 < 0 && clipping_mode != 1u) {
        clipped_columns = (xdb_u16)(0u - x_0);
        raster->remaining = (xdb_i16)(
                (xdb_u16)raster->remaining - clipped_columns);
        raster->edge_0_position = xdb_manu3_add_i32(
                raster->edge_0_position,
                xdb_manu3_multiply_low(raster->edge_0_step, clipped_columns));
        if (clipping_mode != 2u) {
            raster->edge_1_position = xdb_manu3_add_i32(
                    raster->edge_1_position,
                    xdb_manu3_multiply_low(
                            raster->edge_1_step, clipped_columns));
        }
        raster->depth_position = xdb_manu3_add_i32(
                raster->depth_position,
                xdb_manu3_multiply_low(raster->depth_step, clipped_columns));
        raster->texture_u = (xdb_i16)(
                (xdb_u16)raster->texture_u
                + (xdb_u16)raster->texture_u_step * clipped_columns);
        raster->texture_v = (xdb_i16)(
                (xdb_u16)raster->texture_v
                + (xdb_u16)raster->texture_v_step * clipped_columns);
    }

    xdb_manu3_active_raster_offset = raster->next;
    {
        volatile xdb_manu3_raster_record XDB_NEAR *previous =
                (volatile xdb_manu3_raster_record XDB_NEAR *)
                XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET;
        volatile xdb_manu3_raster_record XDB_NEAR *next =
                (volatile xdb_manu3_raster_record XDB_NEAR *)previous->next;

        if (raster->edge_0_position > next->edge_0_position
                || (raster->edge_0_position == next->edge_0_position
                && raster->edge_0_step > next->edge_0_step)) {
            do {
                previous = next;
                next = (volatile xdb_manu3_raster_record XDB_NEAR *)next->next;
            } while (raster->edge_0_position > next->edge_0_position
                    || (raster->edge_0_position == next->edge_0_position
                    && raster->edge_0_step > next->edge_0_position));
        }
        previous->next = (xdb_u16)raster;
        raster->previous = (xdb_u16)previous;
        raster->next = (xdb_u16)next;
        next->previous = (xdb_u16)raster;
    }
}
