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

void XDB_NEAR xdb_scrut_slot2_selection_approach(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value;
    xdb_i16 delta;
    xdb_u16 rounding_carry;

    if ((xdb_scrut_slot1_selection_state & 3u) == 0u) {
        xdb_scrut_slot2_selection_reset_restart(state, context);
        return;
    }

    state->field_04e = sar16(state->field_04e, 1u);
    value = (xdb_i16)(xdb_alien_camera_matrix[6] >> 3);
    value = (xdb_i16)((xdb_u16)value - (xdb_u16)xdb_alien_view_x
            + (xdb_u16)context->continuation.scrut_slot2.signed_seed);
    delta = (xdb_i16)((xdb_u16)value
            - (xdb_u16)*state_word(state, 0x42));
    *state_word(state, 0x42) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x42)
            + (xdb_u16)sar16(delta, 4u));

    value = (xdb_i16)((xdb_u16)xdb_alien_view_y
            + (xdb_u16)*state_word(state, 0x46));
    value = (xdb_i16)-value;
    delta = sar16(value, 5u);
    rounding_carry = (xdb_u16)(((xdb_u16)value >> 4) & 1u);
    *state_word(state, 0x46) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x46)
            + (xdb_u16)delta + rounding_carry);

    value = (xdb_i16)(xdb_alien_camera_matrix[8] >> 3);
    value = (xdb_i16)((xdb_u16)value - (xdb_u16)xdb_alien_view_z
            + (xdb_u16)context->continuation.scrut_slot2.signed_seed);
    delta = (xdb_i16)((xdb_u16)value
            - (xdb_u16)*state_word(state, 0x4a));
    *state_word(state, 0x4a) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x4a)
            + (xdb_u16)sar16(delta, 4u));

    if ((xdb_i16)state->field_040 < 0x012c
            || (xdb_u16)((xdb_u16)state->field_038 + 0x0bb8u)
                    > 0x1770u) {
        xdb_scrut_slot2_selection_restart(state, context);
        return;
    }
    if (xdb_scrut_slot2_steering_helper(state, 0x13u)) {
        xdb_scrut_slot2_finish_setup(state, context);
    }
}
