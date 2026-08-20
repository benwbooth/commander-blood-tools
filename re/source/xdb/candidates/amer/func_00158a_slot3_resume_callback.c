#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot3_resume_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_index;

    (void)context;
    ring_index = (xdb_u16)(state->ring_offset >> 3);
    state->position_x = 0;
    state->position_y = 0x06a4L;
    state->position_z = 0;
    state->field_04e = 0;
    state->field_050 = 0;
    state->field_052 = 0;
    state->field_054 = 0;
    state->callback = xdb_amer_slot3_ring_zero_callback;
    xdb_amer_slot3_ring[ring_index].field_000 = 0;
    xdb_amer_slot3_ring[ring_index].field_002 = 0;
    xdb_amer_slot3_ring[ring_index].field_004 = 0;
    xdb_amer_slot3_ring[ring_index].field_006 = 2;
}
