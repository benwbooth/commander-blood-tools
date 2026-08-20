#include "../include/xdb_alien.h"

static xdb_u16 ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

void XDB_NEAR xdb_amer_slot3_restart_initial_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_index;
    xdb_u16 random_value;

    (void)context;
    ring_index = (xdb_u16)(state->ring_offset >> 3);
    xdb_amer_slot3_ring[ring_index].field_006 = 0;
    xdb_amer_slot3_ring[ring_index].field_004 = 8;
    state->callback = xdb_amer_slot3_initial_update;
    state->field_052 = 0;
    state->field_054 = 8;
    state->field_056 = 0x1e;
    random_value = ror3_sbb_zero(xdb_alien_random_state);
    state->field_05c = random_value;
    xdb_alien_random_state = random_value;
}
