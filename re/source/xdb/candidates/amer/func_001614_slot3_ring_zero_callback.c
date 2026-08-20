#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot3_ring_zero_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_ring_entry XDB_CODE_DATA *ring;
    xdb_u16 ring_cursor;

    (void)context;
    if (xdb_amer_slot3_timer != 0u) {
        return;
    }
    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    ring = &xdb_amer_slot3_ring[ring_cursor >> 3];
    ring->field_000 = 0;
    ring->field_002 = 0;
    ring->field_004 = 0;
    ring->field_006 = 0;
}
