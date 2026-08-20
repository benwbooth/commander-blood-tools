#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_selection_begin(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    if ((xdb_scrut_slot1_selection_state & 3u) == 0u) {
        xdb_scrut_slot2_selection_reset_restart(state, context);
        return;
    }
    if ((xdb_i16)state->field_040 < 0x02bc
            || state->field_038 > 0x01f4
            || state->field_038 > (xdb_i16)0xfe0cu) {
        xdb_scrut_slot2_reset_or_camera(state, context);
        return;
    }

    state->callback = xdb_scrut_slot2_selection_damp;
    state->field_056 = 0x00c8;
    state->ring_offset = (xdb_u16)state->field_052;
    state->field_058 = 0;
    xdb_scrut_slot2_selection_damp(state, context);
}
