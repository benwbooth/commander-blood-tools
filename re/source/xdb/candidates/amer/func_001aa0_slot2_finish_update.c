#include "../include/xdb_alien.h"

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

void XDB_NEAR xdb_amer_slot2_finish_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 countdown = (xdb_i16)(state->field_056 - 1u);
    xdb_i32 horizontal;
    xdb_i32 vertical;
    xdb_i32 score;

    state->field_056 = countdown;
    if (countdown < 0) {
        xdb_amer_slot2_reset(state, context);
        return;
    }

    state->field_04e = clamp_camera_height(sar16((xdb_i16)(
            (xdb_u16)state->field_03c + (xdb_u16)state->field_04e), 1u));
    horizontal = (xdb_i32)(xdb_i16)state->field_040
            - (xdb_u16)xdb_alien_camera_depth_step - 0x03e8L;
    vertical = (xdb_i32)state->field_038;
    if (horizontal >= 0 && horizontal <= 0x03e8L
            && vertical >= -0x03e8L && vertical <= 0x03e8L) {
        state->callback = xdb_amer_slot2_selection_wait;
        state->field_054 = sar16(state->field_054, 1u);
        return;
    }

    state->field_054 = (xdb_i16)(state->field_054 + 0x0a);
    state->field_058 = 0x01f4;
    score = -horizontal * state->field_01a + vertical * state->field_032;
    state->field_050 = (xdb_u16)(state->field_050
            + (score < 0 ? 0x0020u : 0xffe0u));
}
