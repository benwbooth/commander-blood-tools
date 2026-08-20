#include "../include/xdb_alien.h"

static xdb_u16 ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

void XDB_NEAR xdb_scrut_slot2_reset_or_camera(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i32 horizontal;
    xdb_i32 vertical;
    xdb_i32 score;
    xdb_i32 transformed;
    xdb_i16 turn;
    xdb_i16 axis;
    xdb_u16 random_value;
    xdb_u16 depth;

    horizontal = (xdb_i32)(xdb_i16)state->field_040;
    if (horizontal < -0x01f4L) {
        random_value = ror7_sbb_zero(
                context->continuation.scrut_slot2.random_value);
        context->continuation.scrut_slot2.random_value = random_value;
        axis = xdb_alien_angle_table[(random_value & 0x0ffcu) >> 2].cosine;

        transformed = (xdb_i32)axis * xdb_alien_camera_matrix[0];
        transformed += (xdb_i32)axis * xdb_alien_camera_matrix[1];
        *state_word(state, 0x42) = (xdb_i16)(
                (transformed >> 16) - xdb_alien_view_x);
        transformed = (xdb_i32)axis * xdb_alien_camera_matrix[3];
        transformed += (xdb_i32)axis * xdb_alien_camera_matrix[4];
        *state_word(state, 0x46) = (xdb_i16)(
                (transformed >> 16) - xdb_alien_view_y);
        transformed = (xdb_i32)axis * xdb_alien_camera_matrix[6];
        transformed += (xdb_i32)axis * xdb_alien_camera_matrix[7];
        *state_word(state, 0x4a) = (xdb_i16)(
                (transformed >> 16) - xdb_alien_view_z);
        state->field_04e = xdb_alien_camera_pitch;
        state->field_050 = (xdb_u16)xdb_alien_camera_pan;
        state->field_052 = 0;
        depth = (xdb_u16)(xdb_alien_camera_depth_step + 0x012c);
        state->field_054 = (xdb_i16)depth;
        state->field_058 = depth;
        context->continuation.scrut_slot2.duration = 8;
        return;
    }

    horizontal -= 0x07d0L;
    vertical = (xdb_i32)state->field_038
            - context->continuation.scrut_slot2.signed_seed;
    score = -(horizontal * state->field_032 + vertical * state->field_01a);
    score >>= 15;
    if (score < 0) {
        score = ((xdb_i16)state->field_058 >> 2) + 0x10;
    }
    state->field_058 = (xdb_u16)score;

    score = vertical * state->field_032 - horizontal * state->field_01a;
    turn = score < 0 ? 0x10 : (xdb_i16)0xfff0u;
    state->ring_offset = (xdb_u16)turn;
    if (state->field_052 >= 0x0300) {
        state->field_052 = 0x0300;
    } else if (state->field_052 < (xdb_i16)0xfd00u) {
        state->field_052 = (xdb_i16)0xfd00u;
    }
    xdb_scrut_slot2_common_dispatch(state, context);
}
