#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot3_resume_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_ring_entry XDB_CODE_DATA *ring;

    (void)context;
    ring = &xdb_scrut_slot3_ring[state->ring_offset >> 3];
    state->position_x = 0x06a4L;
    state->position_y = 0;
    state->position_z = 0;
    state->field_04e = 0;
    state->field_050 = 0;
    state->field_052 = 0;
    state->field_054 = 0;
    state->callback = xdb_scrut_slot3_ring_zero_callback;
    ring->field_000 = 0;
    ring->field_002 = 0;
    ring->field_004 = 0;
    ring->field_006 = 2;
}
