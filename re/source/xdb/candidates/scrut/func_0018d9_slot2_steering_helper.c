#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

int XDB_NEAR xdb_scrut_slot2_steering_helper(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 shift)
{
    xdb_i32 score;
    xdb_i32 shifted;
    xdb_u16 rounded;
    xdb_i16 value;
    xdb_i16 heading_delta;
    xdb_i16 inverse;
    xdb_alien_biased_state XDB_NEAR *follower;
    xdb_u16 rounding_carry;
    xdb_u16 remaining;

    score = (xdb_i32)state->field_038 * state->field_032
            - (xdb_i32)(xdb_i16)state->field_040 * state->field_01a;
    shifted = score >> shift;
    rounded = (xdb_u16)shifted;
    rounded = (xdb_u16)(rounded
            + (((xdb_u32)score >> (shift - 1u)) & 1u));
    if (rounded == 0u) {
        return 0;
    }

    value = (xdb_i16)(0u - rounded);
    if (value >= 0x20) {
        value = 0x20;
    }
    if (value <= (xdb_i16)0xffe0u) {
        value = (xdb_i16)0xffe0u;
    }
    value = (xdb_i16)((xdb_u16)value + (xdb_u16)state->field_052);
    if ((xdb_i16)(state->ring_offset ^ (xdb_u16)value) < 0) {
        value = sar16(value, 1u);
        state->ring_offset = (xdb_u16)value;
    }
    if (value >= 0x0300) {
        value = 0x0300;
    } else if (value < (xdb_i16)0xfd00u) {
        value = (xdb_i16)0xfd00u;
    }
    state->field_052 = value;
    heading_delta = sar16(value, 5u);
    rounding_carry = (xdb_u16)(((xdb_u16)value >> 4) & 1u);
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)heading_delta + rounding_carry);
    inverse = (xdb_i16)-value;
    follower = state;
    remaining = 5;
    do {
        ++follower;
        follower->field_050 = (xdb_u16)sar16(inverse, 1u);
        follower->field_052 = sar16(inverse, 2u);
    } while (--remaining != 0u);
    return 1;
}
