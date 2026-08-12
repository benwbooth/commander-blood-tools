#include "../include/xdb_alien.h"

volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_amer_method_slot_11_anchor_state(
    const xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_state XDB_NEAR *state;
    volatile xdb_alien_biased_state XDB_NEAR *biased;

    state = context->state;
    biased = (volatile xdb_alien_biased_state XDB_NEAR *)
        ((volatile xdb_u8 XDB_NEAR *)state + XDB_ALIEN_CURSOR_BIAS);
    biased->field_052 = (xdb_i16)(biased->field_052 - XDB_ALIEN_FIELD_DELTA);
    xdb_amer_slot11_cursor = (volatile xdb_u8 XDB_NEAR *)biased;
    return (volatile xdb_u8 XDB_NEAR *)biased;
}
