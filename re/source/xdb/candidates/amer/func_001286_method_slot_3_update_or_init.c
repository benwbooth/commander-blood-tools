#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_method_slot_3_update_or_init(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *state =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_u16 count = context->state_count;
    xdb_u16 ring_cursor;
    xdb_u16 phase;

    if (context->control.state == 0) {
        context->control.state = 1;
        ring_cursor = xdb_amer_slot3_ring_cursor;
        xdb_amer_slot3_timer = 7;

        state->position_x = 0;
        state->position_y = 0x06a4L;
        state->position_z = 0;
        state->callback = xdb_amer_slot3_initial_update;
        state->field_056 = 0x19;
        state->field_058 = 0;
        state->ring_offset = ring_cursor;
        state->field_05c = 0xa957u;
        state->field_04e = 0;
        state->field_050 = 0;
        state->field_052 = 0;
        state->field_054 = 0;
        xdb_amer_slot3_ring[ring_cursor >> 3].field_000 = 0;
        xdb_amer_slot3_ring[ring_cursor >> 3].field_002 = 0;
        xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0x46;
        xdb_amer_slot3_ring[ring_cursor >> 3].field_006 = 0;

        if (--count == 0) {
            xdb_amer_slot3_ring_cursor = (ring_cursor - 8u) & 0x03fcu;
            return;
        }

        ring_cursor -= 8u;
        if (++xdb_amer_slot3_generation != 0) {
            context->control.state = -1;
            state->callback = xdb_amer_slot3_update;
            xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0;
            state->field_04e = 0;
            state->field_050 = 0;
            state->field_052 = 0;
            state->position_x = 0;
            state->position_y = 0x06a4L;
            state->position_z = 0;
        }

        phase = 0;
        do {
            ++state;
            ring_cursor = (ring_cursor - 8u) & 0x03ffu;
            phase += 0x0100u;
            state->callback = xdb_amer_slot3_update;
            state->field_058 = phase;
            state->ring_offset = ring_cursor;
            state->field_05c = 0;
            xdb_amer_slot3_ring[ring_cursor >> 3].field_000 = 0;
            xdb_amer_slot3_ring[ring_cursor >> 3].field_002 = 0;
            xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0;
            xdb_amer_slot3_ring[ring_cursor >> 3].field_006 = 0;
            state->field_04e = 0;
            state->field_050 = 0;
            state->field_052 = 0;
            state->field_054 = 0;
            state->position_x = 0;
            state->position_y = 0x06a4L;
            state->position_z = 0;
        } while (--count != 0);

        xdb_amer_slot3_ring_cursor = (ring_cursor - 8u) & 0x03fcu;
        return;
    }

    if (context->control.state >= 0) {
        --xdb_amer_slot3_timer;
        if (xdb_amer_slot3_timer & 0x8000u) {
            xdb_amer_slot3_timer = 7;
        }
    }

    do {
        state->callback(state, context);
        ++state;
    } while (--count != 0);
}
