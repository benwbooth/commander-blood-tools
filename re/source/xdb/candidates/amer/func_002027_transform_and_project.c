#include "../include/xdb_alien.h"

#define XDB_ALIEN_ANGLE_MASK 0x0ffcu
#define XDB_ALIEN_TRIG(angle) \
    xdb_alien_angle_table[((angle) & XDB_ALIEN_ANGLE_MASK) >> 2]

void XDB_NEAR xdb_amer_transform_and_project(void)
{
    volatile xdb_alien_projection_context XDB_NEAR *context =
            xdb_alien_active_projection_context;
    volatile xdb_alien_projection_state XDB_NEAR *state =
            context->projection_root + 1;
    xdb_u16 geometry_segment = xdb_alien_object_segment;
    xdb_u16 copy_count;

    xdb_alien_projection_remaining = context->state_count;
    do {
        xdb_u16 angle_0 = state->angle_0 & XDB_ALIEN_ANGLE_MASK;
        xdb_u16 angle_1 = state->angle_1 & XDB_ALIEN_ANGLE_MASK;
        xdb_u16 angle_2 = state->angle_2 & XDB_ALIEN_ANGLE_MASK;
        volatile xdb_alien_trig_sample XDB_NEAR *first;
        volatile xdb_alien_trig_sample XDB_NEAR *second;
        volatile xdb_alien_trig_sample XDB_NEAR *base;
        volatile xdb_alien_projection_state XDB_NEAR *parent;
        xdb_i32 value_0;
        xdb_i32 value_1;
        xdb_i32 source_0;
        xdb_i32 source_1;
        xdb_i32 source_2;
        xdb_u16 row;
        xdb_u16 vertex_count;
        xdb_u16 vertex_offset;

        xdb_alien_current_projection_state_offset = (xdb_u16)state;
        xdb_alien_matrix_angle_pan = (xdb_i16)angle_1;
        xdb_alien_matrix_angle_pitch = (xdb_i16)angle_0;
        xdb_alien_matrix_angle_pan_secondary = (xdb_i16)angle_2;

        xdb_alien_rotation_matrix[1][2] = (xdb_i32)(
                0UL - ((xdb_u32)(xdb_i32)XDB_ALIEN_TRIG(angle_0).sine << 1));

        first = &XDB_ALIEN_TRIG(
                (xdb_u16)(angle_0 - angle_1 - angle_2));
        second = &XDB_ALIEN_TRIG(
                (xdb_u16)(angle_0 + angle_1 + angle_2));
        base = &XDB_ALIEN_TRIG((xdb_u16)(angle_1 + angle_2));
        value_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->cosine
                - (xdb_u32)(xdb_i32)second->cosine);
        value_0 >>= 1;
        value_0 = (xdb_i32)(
                (xdb_u32)value_0 + (xdb_u32)(xdb_i32)base->sine);
        value_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->sine
                + (xdb_u32)(xdb_i32)second->sine);
        value_1 >>= 1;
        value_1 = (xdb_i32)(
                (xdb_u32)value_1 + (xdb_u32)(xdb_i32)base->cosine);
        xdb_alien_rotation_matrix[0][1] = value_0;
        xdb_alien_rotation_matrix[2][0] = (xdb_i32)(0UL - (xdb_u32)value_0);
        xdb_alien_rotation_matrix[0][0] = value_1;
        xdb_alien_rotation_matrix[2][1] = value_1;

        first = &XDB_ALIEN_TRIG(
                (xdb_u16)(angle_0 - angle_1 + angle_2));
        second = &XDB_ALIEN_TRIG(
                (xdb_u16)(angle_0 + angle_1 - angle_2));
        base = &XDB_ALIEN_TRIG((xdb_u16)(angle_1 - angle_2));
        value_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->cosine
                - (xdb_u32)(xdb_i32)second->cosine);
        value_0 >>= 1;
        value_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->sine
                + (xdb_u32)(xdb_i32)second->sine);
        value_1 >>= 1;
        source_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)base->sine - (xdb_u32)value_0);
        source_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)base->cosine - (xdb_u32)value_1);
        xdb_alien_rotation_matrix[0][1] = (xdb_i32)(
                (xdb_u32)xdb_alien_rotation_matrix[0][1]
                - (xdb_u32)source_0);
        xdb_alien_rotation_matrix[2][0] = (xdb_i32)(
                (xdb_u32)xdb_alien_rotation_matrix[2][0]
                - (xdb_u32)source_0);
        xdb_alien_rotation_matrix[0][0] = (xdb_i32)(
                (xdb_u32)xdb_alien_rotation_matrix[0][0]
                + (xdb_u32)source_1);
        xdb_alien_rotation_matrix[2][1] = (xdb_i32)(
                (xdb_u32)xdb_alien_rotation_matrix[2][1]
                - (xdb_u32)source_1);

        first = &XDB_ALIEN_TRIG((xdb_u16)(angle_2 + angle_0));
        second = &XDB_ALIEN_TRIG((xdb_u16)(angle_2 - angle_0));
        xdb_alien_rotation_matrix[1][1] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->cosine
                + (xdb_u32)(xdb_i32)second->cosine);
        xdb_alien_rotation_matrix[1][0] = (xdb_i32)(
                0UL - ((xdb_u32)(xdb_i32)first->sine
                + (xdb_u32)(xdb_i32)second->sine));

        first = &XDB_ALIEN_TRIG((xdb_u16)(angle_1 + angle_0));
        second = &XDB_ALIEN_TRIG((xdb_u16)(angle_1 - angle_0));
        xdb_alien_rotation_matrix[2][2] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->cosine
                + (xdb_u32)(xdb_i32)second->cosine);
        xdb_alien_rotation_matrix[0][2] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->sine
                + (xdb_u32)(xdb_i32)second->sine);

        if (state->radial_offset != 0) {
            value_0 = (xdb_i32)(
                    (xdb_u32)xdb_alien_rotation_matrix[0][2]
                    * (xdb_u32)(xdb_i32)state->radial_offset);
            state->local_position[0] = (xdb_i32)(
                    (xdb_u32)state->local_position[0]
                    + (xdb_u32)(value_0 >> 16));

            value_0 = (xdb_i32)(
                    (xdb_u32)xdb_alien_rotation_matrix[1][2]
                    * (xdb_u32)(xdb_i32)state->radial_offset);
            value_0 = (xdb_i32)(
                    (xdb_u32)(value_0 >> 16)
                    + (((xdb_u32)value_0 >> 15) & 1UL));
            state->local_position[1] = (xdb_i32)(
                    (xdb_u32)state->local_position[1] + (xdb_u32)value_0);

            value_0 = (xdb_i32)(
                    (xdb_u32)xdb_alien_rotation_matrix[2][2]
                    * (xdb_u32)(xdb_i32)state->radial_offset);
            state->local_position[2] = (xdb_i32)(
                    (xdb_u32)state->local_position[2]
                    + (xdb_u32)(value_0 >> 16));
        }

        parent = (volatile xdb_alien_projection_state XDB_NEAR *)
                state->parent_offset;
        source_0 = (xdb_i32)(xdb_i16)state->local_position[0];
        source_1 = (xdb_i32)(xdb_i16)state->local_position[1];
        source_2 = (xdb_i32)(xdb_i16)state->local_position[2];
        row = 3u;
        do {
            xdb_u32 accumulator;

            --row;
            accumulator = (xdb_u32)parent->matrix[row][0]
                    * (xdb_u32)source_0;
            accumulator += (xdb_u32)parent->matrix[row][1]
                    * (xdb_u32)source_1;
            accumulator += (xdb_u32)parent->matrix[row][2]
                    * (xdb_u32)source_2;
            accumulator += (xdb_u32)parent->translation[row];
            state->translation[row] = (xdb_i32)accumulator;
        } while (row != 0u);

        for (row = 0; row != 3u; ++row) {
            xdb_i32 parent_1 = parent->matrix[row][1];
            xdb_i32 parent_2 = parent->matrix[row][2];
            xdb_u16 column;

            for (column = 0; column != 3u; ++column) {
                xdb_u32 accumulator = (xdb_u32)parent->matrix[row][0]
                        * (xdb_u32)xdb_alien_rotation_matrix[0][column];

                accumulator += (xdb_u32)parent_1
                        * (xdb_u32)xdb_alien_rotation_matrix[1][column];
                accumulator += (xdb_u32)parent_2
                        * (xdb_u32)xdb_alien_rotation_matrix[2][column];
                state->matrix[row][column] = (xdb_i32)accumulator >> 15;
            }
        }

        vertex_count = state->vertex_count;
        vertex_offset = state->vertex_offset;
        xdb_alien_projection_common_clip = 0x800fu;
        xdb_alien_projection_field_2280 = 0u;
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
            xdb_u32 screen_x_accumulator;
            xdb_u32 screen_y_accumulator;
            xdb_i32 depth;
            xdb_i32 screen_x;
            xdb_i32 screen_y;
            xdb_u16 clip_flags;

            accumulator = (xdb_u32)state->matrix[2][0]
                    * (xdb_u32)object_x;
            accumulator += (xdb_u32)state->matrix[2][1]
                    * (xdb_u32)object_y;
            accumulator += (xdb_u32)state->matrix[2][2]
                    * (xdb_u32)object_z;
            accumulator += (xdb_u32)state->translation[2];
            depth = (xdb_i32)accumulator >> 8;
            vertex->depth = depth;

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

            if (depth > 0) {
                screen_x = (xdb_i32)screen_x_accumulator / depth;
                screen_y = (xdb_i32)screen_y_accumulator / depth;
                clip_flags = 0u;
            } else {
                screen_x = (xdb_i32)screen_x_accumulator >> 12;
                screen_y = (xdb_i32)screen_y_accumulator >> 12;
                clip_flags = 0x8000u;
            }

            screen_y = (xdb_i32)(0UL - (xdb_u32)screen_y);
            screen_x = (xdb_i32)(
                    (xdb_u32)screen_x
                    + (xdb_u32)xdb_alien_screen_center_x);
            if (screen_x < 0) {
                clip_flags = (xdb_u16)(clip_flags | 0x0001u);
                if (screen_x <= -90) {
                    screen_x = -89;
                }
            }
            if (screen_x >= 320) {
                clip_flags = (xdb_u16)((clip_flags & 0xff00u) | 0x0002u);
                if (screen_x >= 410) {
                    screen_x = 409;
                }
            }

            screen_y = (xdb_i32)(
                    (xdb_u32)screen_y
                    + (xdb_u32)xdb_alien_screen_center_y);
            if (screen_y < 0) {
                clip_flags |= 0x0004u;
                if (screen_y <= -150) {
                    screen_y = -149;
                }
            }
            if (screen_y >= 200) {
                clip_flags |= 0x0008u;
                if (screen_y >= 350) {
                    screen_y = 349;
                }
            }

            xdb_alien_projection_common_clip &= clip_flags;
            vertex->clip_flags = clip_flags;
            vertex->screen.position.x = (xdb_i16)screen_x;
            vertex->screen.position.y = (xdb_i16)screen_y;
            vertex_offset = (xdb_u16)(
                    vertex_offset + sizeof(xdb_alien_projection_vertex));
        } while (--vertex_count != 0u);

        if (xdb_alien_projection_common_clip != 0u) {
            vertex_count = state->vertex_count;
            vertex_offset = state->vertex_offset;
            do {
                volatile xdb_alien_projection_vertex XDB_FAR *vertex =
                        XDB_FAR_AT(
                                volatile xdb_alien_projection_vertex,
                                geometry_segment,
                                vertex_offset);

                vertex->clip_flags = 0x00ffu;
                vertex_offset = (xdb_u16)(
                        vertex_offset + sizeof(xdb_alien_projection_vertex));
            } while (--vertex_count != 0u);
        }

        ++state;
    } while (--xdb_alien_projection_remaining != 0u);

    copy_count = context->copy_count;
    if (copy_count != 0u) {
        xdb_u16 destination_offset = context->copy_offset;

        do {
            volatile xdb_alien_projection_vertex XDB_FAR *destination =
                    XDB_FAR_AT(
                            volatile xdb_alien_projection_vertex,
                            geometry_segment,
                            destination_offset);
            volatile xdb_alien_projection_vertex XDB_FAR *source =
                    XDB_FAR_AT(
                            volatile xdb_alien_projection_vertex,
                            geometry_segment,
                            destination->field_004.projection_source_offset);
            xdb_u32 screen = source->screen.packed;
            xdb_i32 depth = source->depth;
            xdb_u16 clip_flags = source->clip_flags;

            destination->screen.packed = screen;
            destination->depth = depth;
            destination->clip_flags = clip_flags;
            destination_offset = (xdb_u16)(
                    destination_offset + sizeof(xdb_alien_projection_vertex));
        } while (--copy_count != 0u);
    }
}

#undef XDB_ALIEN_TRIG
#undef XDB_ALIEN_ANGLE_MASK
