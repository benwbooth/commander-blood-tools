#include "../include/xdb_alien.h"

#include <stdlib.h>

void XDB_NEAR xdb_amer_method_slot_2_dispatch_or_init(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *state =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_u16 value;

    if (context->control.state != 0) {
        state->callback(state, context);
        return;
    }

    value = xdb_alien_random_state;
    value = _rotr(value, 7);
    value += (xdb_i16)value >> 15;
    xdb_alien_random_state = value;
    context->control.state = 1;
    context->continuation.amer_slot2.field_038 = 0;
    context->continuation.amer_slot2.random_value = value;
    state->callback = xdb_amer_slot2_update;
    state->field_050 = value & 0x0ffcu;
    state->field_058 = 0x14;
}
