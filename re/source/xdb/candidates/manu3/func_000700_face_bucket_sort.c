#include "../include/xdb_manu3.h"
#include "../include/xdb_video.h"

void XDB_NEAR xdb_manu3_face_bucket_sort(
        xdb_u16 geometry_segment,
        xdb_u16 raster_segment)
{
    xdb_u16 count = xdb_manu3_face_count;
    xdb_u16 face_offset = xdb_manu3_face_list_offset;
    volatile xdb_u16 XDB_FAR *free_head;
    volatile xdb_u16 XDB_FAR *column_cell;
    volatile xdb_u16 XDB_FAR *framebuffer_column_cell;
    volatile xdb_u16 XDB_FAR *bucket_cursor_cell;
    volatile xdb_u16 XDB_FAR *render_continuation;
    volatile xdb_u16 XDB_FAR *clipped_sort_head;
    volatile xdb_u16 XDB_FAR *active_list_root;
    volatile xdb_u16 XDB_FAR *bucket;
    volatile xdb_u16 XDB_FAR *sort_link;
    volatile xdb_manu3_raster_record XDB_FAR *head;
    volatile xdb_manu3_raster_record XDB_FAR *middle;
    volatile xdb_manu3_raster_record XDB_FAR *tail;
    volatile xdb_manu3_raster_record XDB_FAR *record;
    volatile xdb_manu3_raster_record XDB_FAR *next_record;
    volatile xdb_manu3_raster_record XDB_FAR *span;
    volatile xdb_manu3_raster_record XDB_FAR *edge;
    volatile xdb_manu3_raster_record XDB_FAR *insertion;
    volatile xdb_manu3_span_boundary XDB_FAR *boundary;
    volatile xdb_manu3_span_boundary XDB_FAR *next_boundary;
    xdb_u16 bucket_offset;
    xdb_u16 record_offset;
    xdb_u16 next_offset;
    xdb_u16 column;
    xdb_u16 continuation;
    xdb_i16 cutoff;
    xdb_i32 projected_depth;

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
            if (span_1 < XDB_MANU3_MAX_FACE_WIDTH
                    && span_2 < XDB_MANU3_MAX_FACE_WIDTH) {
                xdb_u16 doubled_x = (xdb_u16)((xdb_u16)x_0 << 1);
                xdb_u16 bucket_offset = XDB_MANU3_BUCKET_HEADS_OFFSET;
                xdb_u16 previous_head;
                volatile xdb_u16 XDB_FAR *bucket;

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

        face_offset = (xdb_u16)(face_offset + sizeof(xdb_manu3_face));
    } while (--count != 0u);

    free_head = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            0x0908u);
    column_cell = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_COLUMN_OFFSET);
    framebuffer_column_cell = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_FRAMEBUFFER_COLUMN_OFFSET);
    bucket_cursor_cell = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_BUCKET_CURSOR_OFFSET);
    render_continuation = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_RENDER_CONTINUATION_OFFSET);
    clipped_sort_head = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_CLIPPED_SORT_HEAD_OFFSET);
    active_list_root = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_ROOT_OFFSET);
    head = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET);
    middle = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_MIDDLE_OFFSET);
    tail = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_TAIL_OFFSET);

    record_offset = XDB_MANU3_RASTER_POOL_OFFSET;
    *free_head = record_offset;
    count = XDB_MANU3_RASTER_POOL_COUNT;
    do {
        record = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record_offset);
        next_offset = (xdb_u16)(record_offset + sizeof(*record));
        record->next = next_offset;
        record_offset = next_offset;
        if (count == 1u) {
            record->next = 0u;
        }
    } while (--count != 0u);

    *bucket_cursor_cell = XDB_MANU3_BUCKET_HEADS_OFFSET;
    *column_cell = 0u;
    bucket_offset = XDB_MANU3_BUCKET_HEADS_OFFSET;
    column = 0u;
    while (column < XDB_MANU3_SCREEN_WIDTH) {
        bucket = XDB_FAR_AT(
                volatile xdb_u16,
                raster_segment,
                bucket_offset);
        if (*bucket != 0u) {
            break;
        }
        ++column;
        bucket_offset = (xdb_u16)(bucket_offset + sizeof(xdb_u16));
    }
    if (column == XDB_MANU3_SCREEN_WIDTH) {
        return;
    }

    head->next = XDB_MANU3_ACTIVE_LIST_MIDDLE_OFFSET;
    head->remaining = 0x014a;
    head->edge_0_position = (xdb_i32)0x80000000ul;
    head->edge_1_position = 0;
    head->edge_0_step = 0;
    head->edge_1_step = 0;
    middle->previous = XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET;
    middle->next = XDB_MANU3_ACTIVE_LIST_TAIL_OFFSET;
    middle->remaining = XDB_MANU3_SCREEN_WIDTH;
    middle->edge_0_position = (xdb_i32)
            ((xdb_u32)XDB_MANU3_SCREEN_HEIGHT << 16);
    middle->edge_1_position = (xdb_i32)0x7fff0000ul;
    middle->flags = 0x8000u;
    tail->edge_0_position = (xdb_i32)0x7ffffffful;
    tail->edge_1_position = (xdb_i32)0x7ffffffful;
    tail->advance_offset = XDB_MANU3_ADVANCE_COLUMN_OFFSET;
    tail->remaining = -1;
    *active_list_root = XDB_MANU3_ACTIVE_LIST_MIDDLE_OFFSET;
    *bucket_cursor_cell = bucket_offset;
    *column_cell = column;

next_column:
    bucket = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            *bucket_cursor_cell);
    face_offset = *bucket;
    *bucket = 0u;
    while (face_offset != 0u && *free_head != 0u) {
        volatile xdb_manu3_face XDB_FAR *face = XDB_FAR_AT(
                volatile xdb_manu3_face,
                geometry_segment,
                face_offset);

        next_offset = face->link;
        xdb_manu3_face_activate(face, raster_segment);
        face_offset = next_offset;
    }

    if (head->next != XDB_MANU3_ACTIVE_LIST_MIDDLE_OFFSET) {
        head->flags = 1u;
        head->output_end = XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET + 0x10u;
        edge = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                head->next);
        while ((xdb_i16)((xdb_u32)edge->edge_1_position >> 16) < 0) {
            edge = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    edge->next);
        }
        head->sort_next = 0u;
        edge->sort_next = 0u;
        span = head;
        boundary = XDB_FAR_AT(
                volatile xdb_manu3_span_boundary,
                raster_segment,
                XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET);

        if ((xdb_i16)((xdb_u32)edge->edge_0_position >> 16) < 0) {
            span = edge;
            boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET + 0x10u);
            *clipped_sort_head = (xdb_u16)FP_OFF(edge);
            projected_depth = (xdb_i32)(
                    (xdb_u32)edge->depth_position
                    + (xdb_u32)(
                            (xdb_i32)(-(xdb_i16)(
                                    (xdb_u32)edge->edge_0_position >> 16))
                            * edge->depth_gradient));
            edge->output_start = (xdb_u16)projected_depth;
            edge->output_end = (xdb_u16)((xdb_u32)projected_depth >> 16);

            for (;;) {
                edge = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        edge->next);
                while ((xdb_i16)((xdb_u32)edge->edge_1_position >> 16) < 0) {
                    edge = XDB_FAR_AT(
                            volatile xdb_manu3_raster_record,
                            raster_segment,
                            edge->next);
                }
                if ((xdb_i16)((xdb_u32)edge->edge_0_position >> 16) >= 0) {
                    break;
                }

                projected_depth = (xdb_i32)(
                        (xdb_u32)edge->depth_position
                        + (xdb_u32)(
                                (xdb_i32)(-(xdb_i16)(
                                        (xdb_u32)edge->edge_0_position >> 16))
                                * edge->depth_gradient));
                edge->output_start = (xdb_u16)projected_depth;
                edge->output_end = (xdb_u16)((xdb_u32)projected_depth >> 16);
                sort_link = clipped_sort_head;
                span = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        *sort_link);
                while (FP_OFF(span) != 0u
                        && projected_depth > (xdb_i32)(
                                (xdb_u32)span->output_start
                                | ((xdb_u32)span->output_end << 16))) {
                    sort_link = &span->sort_next;
                    span = XDB_FAR_AT(
                            volatile xdb_manu3_raster_record,
                            raster_segment,
                            *sort_link);
                }
                *sort_link = (xdb_u16)FP_OFF(edge);
                edge->sort_next = FP_OFF(span) == 0u
                        ? 0u
                        : (xdb_u16)FP_OFF(span);
                span = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        *clipped_sort_head);
            }
            boundary->flags = 0u;
            boundary->source_offset = (xdb_u16)FP_OFF(span);
        } else {
            if (edge == middle) {
                goto finish_span_boundaries;
            }
            boundary->flags = 1u;
            boundary->next_boundary_offset = (xdb_u16)FP_OFF(edge);
            boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    (xdb_u16)FP_OFF(edge));
            span = edge;
            boundary->flags = 0u;
            boundary->source_offset = (xdb_u16)FP_OFF(edge);
            edge = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    edge->next);
        }

build_span_boundaries:
        if ((xdb_i16)((xdb_u32)edge->edge_0_position >> 16)
                >= (xdb_i16)((xdb_u32)edge->edge_1_position >> 16)) {
            edge = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    edge->next);
            goto build_span_boundaries;
        }
        if ((xdb_i16)((xdb_u32)span->edge_1_position >> 16)
                > (xdb_i16)((xdb_u32)edge->edge_0_position >> 16)) {
            goto insert_overlapping_span;
        }
        if ((xdb_i16)((xdb_u32)span->edge_1_position >> 16)
                == (xdb_i16)((xdb_u32)edge->edge_0_position >> 16)) {
            boundary->next_boundary_offset = (xdb_u16)FP_OFF(edge);
            cutoff = (xdb_i16)((xdb_u32)edge->edge_0_position >> 16);
            boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    (xdb_u16)FP_OFF(edge));
            if (edge == middle) {
                goto finish_span_boundaries;
            }
            span = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    span->sort_next);
            while (FP_OFF(span) != 0u
                    && cutoff >= (xdb_i16)(
                            (xdb_u32)span->edge_1_position >> 16)) {
                span = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        span->sort_next);
            }
            if (FP_OFF(span) != 0u) {
                boundary->flags = 0u;
                boundary->source_offset = (xdb_u16)FP_OFF(span);
                goto insert_overlapping_span;
            }
            edge->sort_next = 0u;
            span = edge;
            boundary->flags = 0u;
            boundary->source_offset = (xdb_u16)FP_OFF(edge);
            edge = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    edge->next);
            goto build_span_boundaries;
        }

        boundary->next_boundary_offset = (xdb_u16)(FP_OFF(span) + 0x10u);
        cutoff = (xdb_i16)((xdb_u32)span->edge_1_position >> 16);
        boundary = XDB_FAR_AT(
                volatile xdb_manu3_span_boundary,
                raster_segment,
                (xdb_u16)(FP_OFF(span) + 0x10u));
        span = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                span->sort_next);
        while (FP_OFF(span) != 0u
                && cutoff >= (xdb_i16)(
                        (xdb_u32)span->edge_1_position >> 16)) {
            span = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    span->sort_next);
        }
        if (FP_OFF(span) != 0u) {
            boundary->flags = 0u;
            boundary->source_offset = (xdb_u16)FP_OFF(span);
            goto build_span_boundaries;
        }

        if (edge == middle) {
            goto finish_span_boundaries;
        }
        boundary->flags = 1u;
        boundary->next_boundary_offset = (xdb_u16)FP_OFF(edge);
        boundary = XDB_FAR_AT(
                volatile xdb_manu3_span_boundary,
                raster_segment,
                (xdb_u16)FP_OFF(edge));
        edge->sort_next = 0u;
        span = edge;
        boundary->flags = 0u;
        boundary->source_offset = (xdb_u16)FP_OFF(edge);
        edge = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                edge->next);
        goto build_span_boundaries;

insert_overlapping_span:
        if (edge == middle) {
            boundary->next_boundary_offset = (xdb_u16)FP_OFF(edge);
            boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    (xdb_u16)FP_OFF(edge));
            goto finish_span_boundaries;
        }
        if ((xdb_i16)((xdb_u32)edge->edge_0_position >> 16)
                < (xdb_i16)((xdb_u32)edge->edge_1_position >> 16)) {
            *clipped_sort_head = (xdb_u16)FP_OFF(span);
            sort_link = clipped_sort_head;
            while (FP_OFF(span) != 0u) {
                if (edge->edge_0_position >= span->edge_1_position) {
                    sort_link = &span->sort_next;
                    span = XDB_FAR_AT(
                            volatile xdb_manu3_raster_record,
                            raster_segment,
                            *sort_link);
                    continue;
                }
                {
                    xdb_i32 distance = (xdb_i32)(
                            (xdb_u32)edge->edge_0_position
                            - (xdb_u32)span->edge_0_position);
                    xdb_i16 distance_high = (xdb_i16)(
                            (xdb_u32)distance >> 16);
                    xdb_u16 distance_low = (xdb_u16)distance;
                    xdb_i16 gradient_high = (xdb_i16)(
                            (xdb_u32)span->depth_gradient >> 16);
                    xdb_u16 gradient_low = (xdb_u16)span->depth_gradient;
                    xdb_u32 product;

                    product = (xdb_u32)(
                            (xdb_i32)distance_high * gradient_high) << 16;
                    product += (xdb_u32)(
                            (xdb_i32)distance_high * gradient_low);
                    product += (xdb_u32)(
                            (xdb_i32)gradient_high * distance_low);
                    product += ((xdb_u32)distance_low * gradient_low) >> 16;
                    projected_depth = (xdb_i32)(
                            (xdb_u32)span->depth_position + product);
                }
                if (projected_depth >= edge->depth_position) {
                    break;
                }
                sort_link = &span->sort_next;
                span = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        *sort_link);
            }
            *sort_link = (xdb_u16)FP_OFF(edge);
            edge->sort_next = FP_OFF(span) == 0u
                    ? 0u
                    : (xdb_u16)FP_OFF(span);
            span = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    *clipped_sort_head);
        }
        if (span == edge) {
            boundary->next_boundary_offset = (xdb_u16)FP_OFF(edge);
            boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    (xdb_u16)FP_OFF(edge));
            edge->flags = 0u;
            edge->output_start = (xdb_u16)FP_OFF(edge);
        }
        edge = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                edge->next);
        goto build_span_boundaries;

finish_span_boundaries:
        boundary->flags = 0x8000u;
        continuation = *render_continuation;
        if (continuation != XDB_MANU3_RENDER_FOUR_PLANES_OFFSET
                || (column & 3u) == 0u) {
            xdb_u16 framebuffer_segment;
            xdb_u16 framebuffer_offset;

            if (continuation == XDB_MANU3_RENDER_LINEAR_OFFSET) {
                framebuffer_segment = xdb_manu3_linear_framebuffer_segment;
                framebuffer_offset = column;
            } else {
                framebuffer_segment = xdb_manu3_framebuffer_segment;
                framebuffer_offset = (xdb_u16)(column >> 2);
                if (continuation == XDB_MANU3_RENDER_FOUR_PLANES_OFFSET) {
                    xdb_port_write_u16(0x03c4u, 0x0f02u);
                } else {
                    xdb_port_write_u16(
                            0x03c4u,
                            (xdb_u16)(
                                    0x0002u
                                    | (0x0100u << (column & 3u))));
                }
            }
            *framebuffer_column_cell = framebuffer_offset;
            next_boundary = XDB_FAR_AT(
                    volatile xdb_manu3_span_boundary,
                    raster_segment,
                    XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET);
            while ((next_boundary->flags & 0x8000u) == 0u) {
                xdb_i16 height;
                xdb_i16 relative_coordinate;
                xdb_u16 rows;
                xdb_u16 texture_offset;
                xdb_u16 output_offset;
                xdb_u16 texture_u;
                xdb_u16 texture_v;
                xdb_u8 texel;
                xdb_i16 texture_du;
                xdb_i16 texture_dv;
                xdb_u16 row_stride;

                boundary = next_boundary;
                next_boundary = XDB_FAR_AT(
                        volatile xdb_manu3_span_boundary,
                        raster_segment,
                        boundary->next_boundary_offset);
                if ((boundary->flags & 1u) != 0u) {
                    continue;
                }
                height = (xdb_i16)(
                        (xdb_u16)next_boundary->coordinate
                        - (xdb_u16)boundary->coordinate);
                if (height <= 0) {
                    continue;
                }
                record = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        boundary->source_offset);
                relative_coordinate = (xdb_i16)(
                        (xdb_u16)boundary->coordinate
                        - (xdb_u16)(
                                (xdb_u32)record->edge_0_position >> 16));
                texture_du = record->texture_du;
                texture_dv = record->texture_dv;
                texture_u = (xdb_u16)(
                        (xdb_u16)record->texture_u
                        + (xdb_u16)(texture_du * relative_coordinate));
                texture_v = (xdb_u16)(
                        (xdb_u16)record->texture_v
                        + (xdb_u16)(texture_dv * relative_coordinate));
                row_stride = continuation == XDB_MANU3_RENDER_LINEAR_OFFSET
                        ? XDB_MANU3_SCREEN_WIDTH
                        : 0x0050u;
                output_offset = (xdb_u16)(
                        framebuffer_offset
                        + (xdb_u16)(boundary->coordinate * row_stride));
                rows = (xdb_u8)height;
                if (rows == 0u) {
                    rows = 0x0100u;
                }
                do {
                    texture_offset = (xdb_u16)(
                            (texture_u >> 8)
                            | (texture_v & 0xff00u));
                    texel = *XDB_FAR_AT(
                            const volatile xdb_u8,
                            record->texture_segment,
                            texture_offset);
                    texture_u = (xdb_u16)(texture_u + texture_du);
                    texture_v = (xdb_u16)(texture_v + texture_dv);
                    *XDB_FAR_AT(
                            volatile xdb_u8,
                            framebuffer_segment,
                            output_offset) = texel;
                    output_offset = (xdb_u16)(output_offset + row_stride);
                } while (--rows != 0u);
            }
        }
    }

    record = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            head->next);
advance_one_record:
    record->remaining = (xdb_i16)(
            (xdb_u16)record->remaining - 1u);
    if (record->remaining >= 0) {
        record->texture_u = (xdb_i16)(
                (xdb_u16)record->texture_u
                + (xdb_u16)record->texture_u_step);
        record->texture_v = (xdb_i16)(
                (xdb_u16)record->texture_v
                + (xdb_u16)record->texture_v_step);
        record->edge_0_position = (xdb_i32)(
                (xdb_u32)record->edge_0_position
                + (xdb_u32)record->edge_0_step);
        record->depth_position = (xdb_i32)(
                (xdb_u32)record->depth_position
                + (xdb_u32)record->depth_step);
        record->edge_1_position = (xdb_i32)(
                (xdb_u32)record->edge_1_position
                + (xdb_u32)record->edge_1_step);
        record = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record->next);
        goto advance_one_record;
    }

    if (record->advance_offset == XDB_MANU3_ADVANCE_SECONDARY_OFFSET) {
        record->edge_0_position = record->secondary_edge_position;
        record->edge_0_step = record->secondary_edge_step;
        record->texture_u = record->secondary_texture_u;
        record->texture_v = record->secondary_texture_v;
        record->texture_u_step = record->secondary_texture_u_step;
        record->texture_v_step = record->secondary_texture_v_step;
        record->depth_position = record->secondary_depth_position;
        record->depth_step = record->secondary_depth_step;
        record->remaining = record->secondary_remaining;
        record->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
        record->edge_1_position = (xdb_i32)(
                (xdb_u32)record->edge_1_position
                + (xdb_u32)record->edge_1_step);
        record = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record->next);
        goto advance_one_record;
    }
    if (record->advance_offset == XDB_MANU3_ADVANCE_SWITCH_OFFSET) {
        record->edge_0_position = (xdb_i32)(
                (xdb_u32)record->edge_0_position
                + (xdb_u32)record->edge_0_step);
        record->depth_position = (xdb_i32)(
                (xdb_u32)record->depth_position
                + (xdb_u32)record->depth_step);
        record->texture_u = (xdb_i16)(
                (xdb_u16)record->texture_u
                + (xdb_u16)record->texture_u_step);
        record->texture_v = (xdb_i16)(
                (xdb_u16)record->texture_v
                + (xdb_u16)record->texture_v_step);
        record->edge_1_position = record->secondary_edge_position;
        record->edge_1_step = record->secondary_edge_step;
        record->remaining = record->secondary_remaining;
        record->advance_offset = XDB_MANU3_ADVANCE_REMOVE_OFFSET;
        record = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record->next);
        goto advance_one_record;
    }
    if (record->advance_offset == XDB_MANU3_ADVANCE_REMOVE_OFFSET) {
        xdb_u16 removed_offset = (xdb_u16)FP_OFF(record);
        volatile xdb_manu3_raster_record XDB_FAR *previous = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record->previous);

        next_record = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                record->next);
        previous->next = (xdb_u16)FP_OFF(next_record);
        next_record->previous = (xdb_u16)FP_OFF(previous);
        record->next = *free_head;
        *free_head = removed_offset;
        record = next_record;
        goto advance_one_record;
    }
    if (record->advance_offset != XDB_MANU3_ADVANCE_COLUMN_OFFSET) {
        return;
    }

    ++column;
    if (column >= XDB_MANU3_SCREEN_WIDTH) {
        return;
    }
    *column_cell = column;

    span = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            head->next);
    for (;;) {
        edge = XDB_FAR_AT(
                volatile xdb_manu3_raster_record,
                raster_segment,
                span->next);
        if (edge == tail) {
            break;
        }
        if (span->edge_0_position > edge->edge_0_position) {
            next_record = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    edge->next);
            span->next = (xdb_u16)FP_OFF(next_record);
            next_record->previous = (xdb_u16)FP_OFF(span);
            insertion = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    span->previous);
            while (insertion != head
                    && edge->edge_0_position < insertion->edge_0_position) {
                insertion = XDB_FAR_AT(
                        volatile xdb_manu3_raster_record,
                        raster_segment,
                        insertion->previous);
            }
            next_record = XDB_FAR_AT(
                    volatile xdb_manu3_raster_record,
                    raster_segment,
                    insertion->next);
            insertion->next = (xdb_u16)FP_OFF(edge);
            edge->next = (xdb_u16)FP_OFF(next_record);
            edge->previous = (xdb_u16)FP_OFF(insertion);
            next_record->previous = (xdb_u16)FP_OFF(edge);
        } else {
            span = edge;
        }
    }

    *bucket_cursor_cell = (xdb_u16)(
            *bucket_cursor_cell + sizeof(xdb_u16));
    goto next_column;
}
