#include "../include/xdb_alien.h"

#include <stdlib.h>

void XDB_NEAR xdb_scrut_slot2_initialize(
        xdb_alien_method_context XDB_NEAR *context,
        xdb_alien_biased_state XDB_NEAR *state);

#if defined(__WATCOMC__)
#pragma aux xdb_scrut_slot2_initialize \
        parm [di] [si] modify exact [ax bx cx dx si di bp]
#endif

void XDB_NEAR xdb_scrut_method_slot_2_4_dispatch_or_init(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *state;

    state =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    if (context->control.state != 0) {
        state->callback(state, context);
        return;
    }
    xdb_scrut_slot2_initialize(context, state);
}

void XDB_NEAR xdb_scrut_slot2_initialize(
        xdb_alien_method_context XDB_NEAR *context,
        xdb_alien_biased_state XDB_NEAR *state)
{
    xdb_u16 value;

    value = xdb_alien_random_state;
    value = _rotr(value, 7);
    value += (xdb_i16)value >> 15;
    xdb_alien_random_state = value;
    context->control.state = 1;
    context->continuation.scrut_slot2.duration = 0x32;
    context->continuation.scrut_slot2.signed_seed = xdb_scrut_slot2_seed;
    xdb_scrut_slot2_seed += 0x012c;

    value = _rotr(value, 7);
    value += (xdb_i16)value >> 15;
    context->continuation.scrut_slot2.random_value = value;
    state->field_050 = value & 0x0ffcu;
    state->field_052 = 0;
    state->field_054 = 0;
    state->callback = xdb_scrut_slot2_update;
    state->field_056 = 0;
    state->field_058 = 0;
    state->ring_offset = 0;

    value = context->state_count - 1u;
    do {
        ++state;
        state->field_056 = (xdb_i16)(xdb_u16)state->position_x;
        state->ring_offset = (xdb_u16)state->position_z;
    } while (--value != 0);
}
