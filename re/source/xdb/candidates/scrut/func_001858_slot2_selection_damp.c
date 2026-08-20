#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

void XDB_NEAR xdb_scrut_slot2_selection_damp(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->field_054 = sar16(state->field_054, 1u);
    if (state->field_054 != 0) {
        (void)xdb_scrut_slot2_steering_helper(state, 0x14u);
        return;
    }
    state->callback = xdb_scrut_slot2_selection_approach;
    xdb_scrut_slot2_selection_approach(state, context);
}
