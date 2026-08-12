/* Codegen probe for the MANU3 matrix and parent-node transform loop. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#define FAR far
#define NEAR near
#define ANGLE_MASK 0x0ffcu
#define TRIG(angle) trig_table[((angle) & ANGLE_MASK) >> 2]

typedef struct trig_pair {
    i16 component_0;
    i16 component_1;
} trig_pair;

typedef struct projection_state {
    u16 parent_offset;
    u16 vertex_count;
    u16 field_004;
    u16 vertex_offset;
    u8 field_008[0x0A];
    i32 matrix[3][3];
    i32 translation[3];
    i32 local_position[3];
    u16 angle_0;
    u16 angle_1;
    u16 angle_2;
    i16 radial_offset;
    u8 field_056[0x08];
} projection_state;

extern const volatile trig_pair trig_table[];
extern volatile u16 angle_scratch_1;
extern volatile u16 angle_scratch_0;
extern volatile u16 angle_scratch_2;
extern volatile u16 projection_remaining;
extern volatile u16 projection_state_count;
extern volatile u16 current_state_offset;
extern volatile i32 rotation_matrix[3][3];
extern volatile projection_state projection_states[];

void NEAR xdb_manu3_matrix_build_probe(void)
{
    volatile projection_state NEAR *state = projection_states;

    projection_remaining = projection_state_count;
    do {
        u16 angle_0 = state->angle_0 & ANGLE_MASK;
        u16 angle_1 = state->angle_1 & ANGLE_MASK;
        u16 angle_2 = state->angle_2 & ANGLE_MASK;
        const volatile trig_pair NEAR *first;
        const volatile trig_pair NEAR *second;
        const volatile trig_pair NEAR *base;
        volatile projection_state NEAR *parent;
        i32 value_0;
        i32 value_1;
        i32 source_0;
        i32 source_1;
        i32 source_2;
        u16 row;

        current_state_offset = (u16)state;
        angle_scratch_1 = angle_1;
        angle_scratch_0 = angle_0;
        angle_scratch_2 = angle_2;

        rotation_matrix[1][2] = (i32)(
                0u - ((u32)(i32)TRIG(angle_0).component_1 << 1));

        first = &TRIG((u16)(angle_0 - angle_1 - angle_2));
        second = &TRIG((u16)(angle_0 + angle_1 + angle_2));
        base = &TRIG((u16)(angle_1 + angle_2));
        value_0 = (i32)(
                (u32)(i32)first->component_0
                - (u32)(i32)second->component_0);
        value_0 >>= 1;
        value_0 = (i32)((u32)value_0 + (u32)(i32)base->component_1);
        value_1 = (i32)(
                (u32)(i32)first->component_1
                + (u32)(i32)second->component_1);
        value_1 >>= 1;
        value_1 = (i32)((u32)value_1 + (u32)(i32)base->component_0);
        rotation_matrix[0][1] = value_0;
        rotation_matrix[2][0] = (i32)(0u - (u32)value_0);
        rotation_matrix[0][0] = value_1;
        rotation_matrix[2][1] = value_1;

        first = &TRIG((u16)(angle_0 - angle_1 + angle_2));
        second = &TRIG((u16)(angle_0 + angle_1 - angle_2));
        base = &TRIG((u16)(angle_1 - angle_2));
        value_0 = (i32)(
                (u32)(i32)first->component_0
                - (u32)(i32)second->component_0);
        value_0 >>= 1;
        value_1 = (i32)(
                (u32)(i32)first->component_1
                + (u32)(i32)second->component_1);
        value_1 >>= 1;
        source_0 = (i32)((u32)(i32)base->component_1 - (u32)value_0);
        source_1 = (i32)((u32)(i32)base->component_0 - (u32)value_1);
        rotation_matrix[0][1] = (i32)(
                (u32)rotation_matrix[0][1] - (u32)source_0);
        rotation_matrix[2][0] = (i32)(
                (u32)rotation_matrix[2][0] - (u32)source_0);
        rotation_matrix[0][0] = (i32)(
                (u32)rotation_matrix[0][0] + (u32)source_1);
        rotation_matrix[2][1] = (i32)(
                (u32)rotation_matrix[2][1] - (u32)source_1);

        first = &TRIG((u16)(angle_2 + angle_0));
        second = &TRIG((u16)(angle_2 - angle_0));
        rotation_matrix[1][1] = (i32)(
                (u32)(i32)first->component_0
                + (u32)(i32)second->component_0);
        rotation_matrix[1][0] = (i32)(
                0u - ((u32)(i32)first->component_1
                + (u32)(i32)second->component_1));

        first = &TRIG((u16)(angle_1 + angle_0));
        second = &TRIG((u16)(angle_1 - angle_0));
        rotation_matrix[2][2] = (i32)(
                (u32)(i32)first->component_0
                + (u32)(i32)second->component_0);
        rotation_matrix[0][2] = (i32)(
                (u32)(i32)first->component_1
                + (u32)(i32)second->component_1);

        state->local_position[0] = (i32)(
                (u32)state->local_position[0]
                + (u32)((i32)((u32)rotation_matrix[0][2]
                * (u32)(i32)state->radial_offset) >> 16));
        value_0 = (i32)(
                (u32)rotation_matrix[1][2]
                * (u32)(i32)state->radial_offset);
        value_0 = (i32)(
                ((u32)((i32)value_0 >> 16))
                + (((u32)value_0 >> 15) & 1u));
        state->local_position[1] = (i32)(
                (u32)state->local_position[1] + (u32)value_0);
        state->local_position[2] = (i32)(
                (u32)state->local_position[2]
                + (u32)((i32)((u32)rotation_matrix[2][2]
                * (u32)(i32)state->radial_offset) >> 16));

        parent = (volatile projection_state NEAR *)state->parent_offset;
        source_0 = (i32)(i16)state->local_position[0];
        source_1 = (i32)(i16)state->local_position[1];
        source_2 = (i32)(i16)state->local_position[2];
        row = 3u;
        do {
            u32 accumulator;

            --row;
            accumulator = (u32)parent->matrix[row][0] * (u32)source_0;
            accumulator += (u32)parent->matrix[row][1] * (u32)source_1;
            accumulator += (u32)parent->matrix[row][2] * (u32)source_2;
            accumulator += (u32)parent->translation[row];
            state->translation[row] = (i32)accumulator;
        } while (row != 0u);

        for (row = 0; row != 3u; ++row) {
            i32 parent_1 = parent->matrix[row][1];
            i32 parent_2 = parent->matrix[row][2];
            u16 column;

            for (column = 0; column != 3u; ++column) {
                u32 accumulator = (u32)parent->matrix[row][0]
                        * (u32)rotation_matrix[0][column];

                accumulator += (u32)parent_1
                        * (u32)rotation_matrix[1][column];
                accumulator += (u32)parent_2
                        * (u32)rotation_matrix[2][column];
                state->matrix[row][column] = (i32)accumulator >> 15;
            }
        }

        ++state;
    } while (--projection_remaining != 0u);
}

#undef TRIG
#undef ANGLE_MASK
