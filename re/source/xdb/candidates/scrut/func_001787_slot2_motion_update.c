#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 clamp_height(xdb_i16 value)
{
    if (value >= 0x0300) {
        return 0x0300;
    }
    if (value <= (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    return value;
}

void XDB_NEAR xdb_scrut_slot2_motion_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *follower;
    xdb_i16 value;
    xdb_i16 heading_delta;
    xdb_u16 rounding_carry;
    xdb_u16 remaining;

    value = (xdb_i16)((xdb_u16)state->field_03c
            + (xdb_u16)context->continuation.scrut_slot2.signed_seed);
    value = sar16(value, 1u);
    value = (xdb_i16)((xdb_u16)value - (xdb_u16)state->field_04e);
    state->field_04e = clamp_height((xdb_i16)(state->field_04e
            + sar16(value, 3u)));

    value = (xdb_i16)(state->field_058 - (xdb_u16)state->field_054);
    state->field_054 = (xdb_i16)(state->field_054 + sar16(value, 3u));
    value = (xdb_i16)((xdb_u16)(xdb_i16)state->ring_offset
            + (xdb_u16)state->field_052);
    state->field_052 = value;
    heading_delta = sar16(value, 5u);
    rounding_carry = (xdb_u16)(((xdb_u16)value >> 4) & 1u);
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)heading_delta + rounding_carry);

    value = (xdb_i16)-value;
    follower = state;
    remaining = 5;
    do {
        ++follower;
        follower->field_050 = (xdb_u16)sar16(value, 1u);
        follower->field_052 = sar16(value, 2u);
    } while (--remaining != 0u);
}
