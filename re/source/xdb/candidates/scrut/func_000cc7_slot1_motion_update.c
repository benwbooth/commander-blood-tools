#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot1_motion_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    state->field_050 = (xdb_u16)(state->field_050 + (xdb_u16)state->field_056);
    state->field_052 = (xdb_i16)(
            (xdb_u16)state->field_052 - (xdb_u16)state->field_010);
    state->field_054 = (xdb_i16)((xdb_u16)state->field_054 + 1u);
    if (state->field_054 <= 0x000f) {
        return;
    }
    state->callback = xdb_scrut_slot1_return_update;
    state->field_054 = 0x0040;
}
