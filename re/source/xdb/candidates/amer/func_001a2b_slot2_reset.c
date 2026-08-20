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

void XDB_NEAR xdb_amer_slot2_reset(
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
    context->continuation.amer_slot2.random_value = random_value;
    state->field_04e = sar16((xdb_i16)random_value, 6u);
    state->field_056 = 0x20;
}
