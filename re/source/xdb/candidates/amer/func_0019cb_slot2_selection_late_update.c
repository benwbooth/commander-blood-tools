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

void XDB_NEAR xdb_amer_slot2_selection_late_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i32 score;

    if (state->field_040 > 0x03e8u) {
        xdb_amer_slot2_selection_wait(state, context);
        return;
    }
    if (state->field_038 > 0x01f4
            || state->field_038 < (xdb_i16)0xfe0cu) {
        xdb_amer_slot2_reset(state, context);
        return;
    }

    score = -(xdb_i32)((xdb_i16)state->field_040 - 0x00c8)
                    * state->field_01a
            + (xdb_i32)state->field_038 * state->field_032;
    context->continuation.amer_slot2_motion.velocity_x =
            (xdb_i16)(score < 0 ? 0x0030 : (xdb_i16)0xffd0u);
    state->field_04e = clamp_camera_height(sar16((xdb_i16)(
            (xdb_u16)state->position_y + (xdb_u16)xdb_alien_view_y
            + (xdb_u16)state->field_04e), 1u));
    xdb_amer_slot2_common_update(state, context);
}
