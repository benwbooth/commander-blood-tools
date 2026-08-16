#include "../include/xdb_alien.h"

static xdb_u16 ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 clamp_camera_height(xdb_i16 value)
{
    if (value < (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    if (value > 0x0300) {
        return 0x0300;
    }
    return value;
}

static void XDB_NEAR slot2_invalid_reset(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 random_value = ror7_sbb_zero(
            context->continuation.amer_slot2.random_value);

    state->field_052 = 0;
    context->continuation.amer_slot2_motion.velocity_x = 0;
    state->field_05c = 0;
    state->field_054 = 0x3c;
    state->callback = xdb_amer_slot2_steer_update;
    context->continuation.amer_slot2_motion.velocity_x =
            sar16((xdb_i16)random_value, 6u);
    context->continuation.amer_slot2.random_value = random_value;
    state->field_056 = 0x20;
}

void XDB_NEAR xdb_amer_slot2_finish_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 countdown = (xdb_i16)((xdb_u16)state->field_056 - 1u);
    xdb_i16 position;
    xdb_i32 first_factor;
    xdb_i32 score;

    state->field_056 = countdown;
    if (countdown < 0) {
        slot2_invalid_reset(state, context);
        return;
    }

    position = (xdb_i16)((xdb_u16)state->field_03c
            + (xdb_u16)state->field_04e);
    state->field_04e = clamp_camera_height(sar16(position, 1u));
    first_factor = (xdb_i32)(xdb_i16)state->field_040
            - (xdb_i32)xdb_alien_camera_depth_step - 0x03e8L;
    first_factor = -first_factor;
    score = first_factor * state->field_01a
            + (xdb_i32)state->field_038 * state->field_032;
    state->field_050 = (xdb_u16)(state->field_050
            + (score < 0 ? 0x0020u : 0xffe0u));
}
