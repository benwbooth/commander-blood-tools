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

void XDB_NEAR xdb_scrut_slot2_finish_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value;

    xdb_scrut_slot2_selection_value = 0x03e8;
    *state_word(state, 0x033a) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x033a) + 0x40u);
    *state_word(state, 0x0398) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x0398) + 0x50u);
    *state_word(state, 0x0332) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x0332) + 4u);
    *state_word(state, 0x0390) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x0390) - 4u);

    value = (xdb_i16)state->field_040;
    if (value >= 0x01f4) {
        value = (xdb_i16)(0x00c8u - (xdb_u16)state->field_054);
        state->field_054 = (xdb_i16)(state->field_054
                + sar16(value, 4u));
        if (xdb_alien_control_latch == (xdb_u16)(size_t)context) {
            xdb_scrut_slot2_active = 1;
        }
        return;
    }
    if (value >= 0) {
        state->field_04e = (xdb_i16)(state->field_04e - 0x20);
        return;
    }

    xdb_scrut_slot2_selection_value = 0;
    *state_word(state, 0x0332) = *state_word(state, 0x0346);
    *state_word(state, 0x033a) = *state_word(state, 0x034a);
    *state_word(state, 0x0390) = *state_word(state, 0x03a4);
    *state_word(state, 0x0398) = *state_word(state, 0x03a8);
    xdb_scrut_slot2_active = 0;
    xdb_scrut_slot2_selection_init(state, context);
}
