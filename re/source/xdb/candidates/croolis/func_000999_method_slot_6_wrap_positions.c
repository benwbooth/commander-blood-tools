#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *state =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_u16 count = context->state_count;
    xdb_u16 value;

    do {
        value = (xdb_u16)state->position_x + (xdb_u16)xdb_alien_view_x;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_x = (xdb_i16)(value - (xdb_u16)xdb_alien_view_x);

        value = (xdb_u16)state->position_y + (xdb_u16)xdb_alien_view_y;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_y = (xdb_i16)(value - (xdb_u16)xdb_alien_view_y);

        value = (xdb_u16)state->position_z + (xdb_u16)xdb_alien_view_z;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_z = (xdb_i16)(value - (xdb_u16)xdb_alien_view_z);

        ++state;
    } while (--count != 0u);
}
