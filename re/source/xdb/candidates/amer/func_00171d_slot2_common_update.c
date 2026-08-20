#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

static volatile xdb_u16 XDB_NEAR *context_word(
        xdb_alien_method_context XDB_NEAR *context,
        xdb_u16 offset)
{
    return (volatile xdb_u16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)context + offset);
}

void XDB_NEAR xdb_amer_slot2_common_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value = (xdb_i16)(state->field_058 - state->field_054);

    state->field_054 = (xdb_i16)(state->field_054 + sar16(value, 3u));
    if (state->field_038 >= (xdb_i16)0xffd8u
            && state->field_038 < 0x28
            && state->field_03c >= (xdb_i16)0xffd8u
            && state->field_03c <= 0x28
            && (xdb_i16)state->field_040 <= 0x28
            && (xdb_i16)state->field_040 > (xdb_i16)0xffb0u) {
        xdb_i16 depth;
        xdb_i16 scale;

        if ((xdb_i16)state->field_040 < 0) {
            state->field_050 = (xdb_u16)(xdb_alien_camera_pan + 0x0800u)
                    & 0x0ffcu;
            return;
        }

        state->field_054 = 0;
        state->position_x = -(xdb_i32)(
                ((xdb_i32)xdb_alien_camera_matrix[6] * -0x00a0
                 + xdb_alien_camera_position[0]) >> 16);
        state->position_y = -(xdb_i32)(
                ((xdb_i32)xdb_alien_camera_matrix[7] * -0x00a0
                 + xdb_alien_camera_position[1]) >> 16);
        state->position_z = -(xdb_i32)(
                ((xdb_i32)xdb_alien_camera_matrix[8] * -0x00a0
                 + xdb_alien_camera_position[2]) >> 16);
        depth = sar16(xdb_alien_camera_depth_step, 1u);
        if (depth < 0x28) {
            depth = 0x28;
        }
        scale = (xdb_i16)(((xdb_u16)depth >> 2) + 0x14u);
        context->continuation.amer_slot2_motion.countdown = depth;
        context->continuation.amer_slot2_motion.velocity_x =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[6] * scale) >> 18);
        context->continuation.amer_slot2_motion.velocity_y =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[7] * scale) >> 18);
        context->continuation.amer_slot2_motion.velocity_z =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[8] * scale) >> 18);
        xdb_alien_camera_depth_step = (xdb_i16)0xffc0u;
        context->control.state = (xdb_i16)0x8001u;
        state->callback = xdb_amer_slot2_return_update;
        xdb_amer_slot2_active = 1;
        xdb_alien_callback_countdown = 1;
        return;
    }

    value = (xdb_i16)(
            context->continuation.amer_slot2_motion.velocity_x
            + state->field_052);
    state->field_052 = value;
    state->field_050 = (xdb_u16)(state->field_050 + sar16(value, 3u));
    *context_word(context, 0x42) =
            (xdb_u16)((*context_word(context, 0x42) + 0x84u) & 0x03ffu);

    value = (xdb_i16)(state->field_052 + 0x20);
    if (value < 0) {
        *state_word(state, 0x0ac) = (xdb_i16)0xff00u;
        *state_word(state, 0x0b0) = (xdb_i16)state->position_x;
        *state_word(state, 0x10a) = 0x0100;
        *state_word(state, 0x10e) = (xdb_i16)(-state->position_x);
        *state_word(state, 0x168) = (xdb_i16)0xff00u;
        *state_word(state, 0x16c) = (xdb_i16)(-state->position_x);
        *state_word(state, 0x1c6) = 0x0100;
        *state_word(state, 0x1ca) = (xdb_i16)state->position_x;
        return;
    }
    if (value < 0x40) {
        *state_word(state, 0x0ac) = (xdb_i16)0xfe00u;
        *state_word(state, 0x0b0) = 0;
        *state_word(state, 0x10a) = 0x0200;
        *state_word(state, 0x10e) = 0;
        *state_word(state, 0x168) = (xdb_i16)0xff00u;
        *state_word(state, 0x16c) = (xdb_i16)(-state->position_x);
        *state_word(state, 0x1c6) = 0x0100;
        *state_word(state, 0x1ca) = (xdb_i16)(-state->position_x);
        return;
    }
    *state_word(state, 0x0ac) = (xdb_i16)0xff00u;
    *state_word(state, 0x0b0) = (xdb_i16)state->position_x;
    *state_word(state, 0x10a) = 0x0100;
    *state_word(state, 0x10e) = (xdb_i16)(-state->position_x);
    *state_word(state, 0x168) = (xdb_i16)0xfe00u;
    *state_word(state, 0x16c) = 0;
    *state_word(state, 0x1c6) = 0x0200;
    *state_word(state, 0x1ca) = 0;
}
