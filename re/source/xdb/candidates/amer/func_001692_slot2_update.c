#include "../include/xdb_alien.h"

static xdb_u16 ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
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

void XDB_NEAR xdb_amer_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 countdown;
    xdb_i16 denominator;
    xdb_i16 numerator;
    xdb_u16 random_value;

    if (xdb_alien_method_delta >= 0
            && (xdb_amer_slot1_selection_state & 1u) != 0u) {
        xdb_amer_slot2_selection_wait(state, context);
        return;
    }

    countdown = (xdb_i16)(context->continuation.amer_slot2.field_038 - 1u);
    context->continuation.amer_slot2.field_038 = countdown;
    if (countdown >= 0) {
        xdb_amer_slot2_common_update(state, context);
        return;
    }

    if ((xdb_i16)state->field_040 > 0x05dc
            || (xdb_i16)state->field_040 < (xdb_i16)0xfc18u
            || state->field_038 > 0x05dc
            || state->field_038 < (xdb_i16)0xfa24u) {
        xdb_amer_slot2_reset(state, context);
        return;
    }

    random_value = ror3_sbb_zero(
            context->continuation.amer_slot2.random_value);
    numerator = (xdb_i16)((random_value & 0x07ffu) - 0x03ffu);
    denominator = numerator;
    if (denominator < 0) {
        denominator = (xdb_i16)(-denominator);
    }
    denominator = (xdb_i16)(((xdb_u16)denominator >> 2) + 0x10u);
    context->continuation.amer_slot2.field_038 = denominator;
    numerator = (xdb_i16)(numerator - state->field_052);
    context->continuation.amer_slot2.random_value = random_value;
    context->continuation.amer_slot2_motion.velocity_x =
            (xdb_i16)(numerator / denominator);
    state->field_058 = 0x14;
    state->field_04e = clamp_camera_height(sar16((xdb_i16)(
            (xdb_u16)state->field_03c + (xdb_u16)state->field_04e), 1u));
    xdb_amer_slot2_common_update(state, context);
}
