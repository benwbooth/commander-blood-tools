#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot3_ring_zero_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor;
    xdb_u16 ring_index;

    (void)context;
    if (xdb_scrut_slot3_timer != 0u) {
        return;
    }
    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    ring_index = (xdb_u16)(ring_cursor >> 3);
    xdb_scrut_slot3_ring[ring_index].field_000 = 0;
    xdb_scrut_slot3_ring[ring_index].field_002 = 0;
    xdb_scrut_slot3_ring[ring_index].field_004 = 0;
    xdb_scrut_slot3_ring[ring_index].field_006 = 0;
}
