#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

void XDB_NEAR xdb_amer_slot2_common_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 radial_delta = (xdb_i16)(
            (xdb_u16)state->field_058 - (xdb_u16)state->field_054);

    state->field_054 = (xdb_i16)(
            (xdb_u16)state->field_054
            + (xdb_u16)sar16(radial_delta, 3u));
    if (state->field_038 >= (xdb_i16)0xffd8u
            && state->field_038 < 0x28
            && state->field_03c >= (xdb_i16)0xffd8u
            && state->field_03c <= 0x28
            && (xdb_i16)state->field_040 <= 0x28
            && (xdb_i16)state->field_040 > (xdb_i16)0xffb0u) {
        xdb_i16 return_depth;
        xdb_u16 return_timer;

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
        return_depth = sar16(xdb_alien_camera_depth_step, 1u);
        if (return_depth <= 0x28) {
            return_depth = 0x28;
        }
        return_timer = (xdb_u16)(
                ((xdb_u16)return_depth >> 2) + 0x14u);
        context->continuation.amer_slot2_motion.countdown =
                (xdb_i16)return_timer;
        context->continuation.amer_slot2_motion.velocity_x =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[6]
                           * return_depth) >> 18);
        context->continuation.amer_slot2_motion.velocity_y =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[7]
                           * return_depth) >> 18);
        context->continuation.amer_slot2_motion.velocity_z =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[8]
                           * return_depth) >> 18);
        xdb_alien_camera_depth_step = (xdb_i16)0xffc0u;
        context->control.state = (xdb_i16)0x8001u;
        state->callback = xdb_amer_slot2_return_update;
        xdb_amer_slot2_active = 1;
        xdb_alien_callback_countdown = 1;
        return;
    }

    {
        xdb_i16 roll = (xdb_i16)(
                (xdb_u16)context->continuation.amer_slot2_motion.velocity_x
                + (xdb_u16)state->field_052);
        xdb_i16 roll_region;
        xdb_u16 phase;
        xdb_u16 negative_phase;

        state->field_052 = roll;
        state->field_050 = (xdb_u16)(
                state->field_050 + (xdb_u16)sar16(roll, 3u));
        phase = (xdb_u16)(
                (context->continuation.amer_slot2_motion.animation_phase
                 + 0x84u)
                & 0x03ffu);
        context->continuation.amer_slot2_motion.animation_phase = phase;
        negative_phase = (xdb_u16)(0u - phase);

        roll_region = (xdb_i16)((xdb_u16)roll + 0x20u);
        if (roll_region < 0) {
            state[1].field_04e = (xdb_i16)0xff00u;
            state[1].field_052 = (xdb_i16)phase;
            state[2].field_04e = 0x0100;
            state[2].field_052 = (xdb_i16)negative_phase;
            state[3].field_04e = (xdb_i16)0xfe00u;
            state[3].field_052 = 0;
            state[4].field_04e = 0x0200;
            state[4].field_052 = 0;
            return;
        }
        roll_region = (xdb_i16)((xdb_u16)roll_region - 0x40u);
        if (roll_region >= 0) {
            state[1].field_04e = (xdb_i16)0xfe00u;
            state[1].field_052 = 0;
            state[2].field_04e = 0x0200;
            state[2].field_052 = 0;
            state[3].field_04e = (xdb_i16)0xff00u;
            state[3].field_052 = (xdb_i16)negative_phase;
            state[4].field_04e = 0x0100;
            state[4].field_052 = (xdb_i16)phase;
            return;
        }
        state[1].field_04e = (xdb_i16)0xff00u;
        state[1].field_052 = (xdb_i16)phase;
        state[2].field_04e = 0x0100;
        state[2].field_052 = (xdb_i16)negative_phase;
        state[3].field_04e = (xdb_i16)0xff00u;
        state[3].field_052 = (xdb_i16)negative_phase;
        state[4].field_04e = 0x0100;
        state[4].field_052 = (xdb_i16)phase;
        return;
    }
}
