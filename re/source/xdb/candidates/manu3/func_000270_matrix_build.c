#include "../include/xdb_manu3.h"

#define XDB_MANU3_ANGLE_MASK 0x0ffcu
#define XDB_MANU3_TRIG(angle) \
    xdb_manu3_trig_table[((angle) & XDB_MANU3_ANGLE_MASK) >> 2]

void XDB_NEAR xdb_manu3_matrix_build(void)
{
    volatile xdb_manu3_projection_state XDB_NEAR *state =
            xdb_manu3_projection_states;

    xdb_manu3_projection_remaining = xdb_manu3_projection_state_count;
    do {
        xdb_u16 angle_0 = state->angle_0 & XDB_MANU3_ANGLE_MASK;
        xdb_u16 angle_1 = state->angle_1 & XDB_MANU3_ANGLE_MASK;
        xdb_u16 angle_2 = state->angle_2 & XDB_MANU3_ANGLE_MASK;
        const volatile xdb_manu3_trig_pair XDB_NEAR *first;
        const volatile xdb_manu3_trig_pair XDB_NEAR *second;
        const volatile xdb_manu3_trig_pair XDB_NEAR *base;
        volatile xdb_manu3_projection_state XDB_NEAR *parent;
        xdb_i32 value_0;
        xdb_i32 value_1;
        xdb_i32 source_0;
        xdb_i32 source_1;
        xdb_i32 source_2;
        xdb_u16 row;

        xdb_manu3_current_state_offset = (xdb_u16)state;
        xdb_manu3_angle_scratch_1 = angle_1;
        xdb_manu3_angle_scratch_0 = angle_0;
        xdb_manu3_angle_scratch_2 = angle_2;

        xdb_manu3_rotation_matrix[1][2] = (xdb_i32)(
                0u - ((xdb_u32)(xdb_i32)XDB_MANU3_TRIG(angle_0).component_1
                << 1));

        first = &XDB_MANU3_TRIG(
                (xdb_u16)(angle_0 - angle_1 - angle_2));
        second = &XDB_MANU3_TRIG(
                (xdb_u16)(angle_0 + angle_1 + angle_2));
        base = &XDB_MANU3_TRIG((xdb_u16)(angle_1 + angle_2));
        value_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_0
                - (xdb_u32)(xdb_i32)second->component_0);
        value_0 >>= 1;
        value_0 = (xdb_i32)(
                (xdb_u32)value_0 + (xdb_u32)(xdb_i32)base->component_1);
        value_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_1
                + (xdb_u32)(xdb_i32)second->component_1);
        value_1 >>= 1;
        value_1 = (xdb_i32)(
                (xdb_u32)value_1 + (xdb_u32)(xdb_i32)base->component_0);
        xdb_manu3_rotation_matrix[0][1] = value_0;
        xdb_manu3_rotation_matrix[2][0] = (xdb_i32)(0u - (xdb_u32)value_0);
        xdb_manu3_rotation_matrix[0][0] = value_1;
        xdb_manu3_rotation_matrix[2][1] = value_1;

        first = &XDB_MANU3_TRIG(
                (xdb_u16)(angle_0 - angle_1 + angle_2));
        second = &XDB_MANU3_TRIG(
                (xdb_u16)(angle_0 + angle_1 - angle_2));
        base = &XDB_MANU3_TRIG((xdb_u16)(angle_1 - angle_2));
        value_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_0
                - (xdb_u32)(xdb_i32)second->component_0);
        value_0 >>= 1;
        value_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_1
                + (xdb_u32)(xdb_i32)second->component_1);
        value_1 >>= 1;
        source_0 = (xdb_i32)(
                (xdb_u32)(xdb_i32)base->component_1 - (xdb_u32)value_0);
        source_1 = (xdb_i32)(
                (xdb_u32)(xdb_i32)base->component_0 - (xdb_u32)value_1);
        xdb_manu3_rotation_matrix[0][1] = (xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[0][1]
                - (xdb_u32)source_0);
        xdb_manu3_rotation_matrix[2][0] = (xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[2][0]
                - (xdb_u32)source_0);
        xdb_manu3_rotation_matrix[0][0] = (xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[0][0]
                + (xdb_u32)source_1);
        xdb_manu3_rotation_matrix[2][1] = (xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[2][1]
                - (xdb_u32)source_1);

        first = &XDB_MANU3_TRIG((xdb_u16)(angle_2 + angle_0));
        second = &XDB_MANU3_TRIG((xdb_u16)(angle_2 - angle_0));
        xdb_manu3_rotation_matrix[1][1] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_0
                + (xdb_u32)(xdb_i32)second->component_0);
        xdb_manu3_rotation_matrix[1][0] = (xdb_i32)(
                0u - ((xdb_u32)(xdb_i32)first->component_1
                + (xdb_u32)(xdb_i32)second->component_1));

        first = &XDB_MANU3_TRIG((xdb_u16)(angle_1 + angle_0));
        second = &XDB_MANU3_TRIG((xdb_u16)(angle_1 - angle_0));
        xdb_manu3_rotation_matrix[2][2] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_0
                + (xdb_u32)(xdb_i32)second->component_0);
        xdb_manu3_rotation_matrix[0][2] = (xdb_i32)(
                (xdb_u32)(xdb_i32)first->component_1
                + (xdb_u32)(xdb_i32)second->component_1);

        state->local_position[0] = (xdb_i32)(
                (xdb_u32)state->local_position[0]
                + (xdb_u32)((xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[0][2]
                * (xdb_u32)(xdb_i32)state->radial_offset) >> 16));
        value_0 = (xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[1][2]
                * (xdb_u32)(xdb_i32)state->radial_offset);
        value_0 = (xdb_i32)(
                ((xdb_u32)((xdb_i32)value_0 >> 16))
                + (((xdb_u32)value_0 >> 15) & 1u));
        state->local_position[1] = (xdb_i32)(
                (xdb_u32)state->local_position[1] + (xdb_u32)value_0);
        state->local_position[2] = (xdb_i32)(
                (xdb_u32)state->local_position[2]
                + (xdb_u32)((xdb_i32)(
                (xdb_u32)xdb_manu3_rotation_matrix[2][2]
                * (xdb_u32)(xdb_i32)state->radial_offset) >> 16));

        parent = (volatile xdb_manu3_projection_state XDB_NEAR *)
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
                        * (xdb_u32)xdb_manu3_rotation_matrix[0][column];

                accumulator += (xdb_u32)parent_1
                        * (xdb_u32)xdb_manu3_rotation_matrix[1][column];
                accumulator += (xdb_u32)parent_2
                        * (xdb_u32)xdb_manu3_rotation_matrix[2][column];
                state->matrix[row][column] = (xdb_i32)accumulator >> 15;
            }
        }

        ++state;
    } while (--xdb_manu3_projection_remaining != 0u);
}

#undef XDB_MANU3_TRIG
#undef XDB_MANU3_ANGLE_MASK
