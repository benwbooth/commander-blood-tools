#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_method_slot_10_bounds_then_wrap(
        xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_biased_state XDB_NEAR *state =
            (volatile xdb_alien_biased_state XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)context->state +
             XDB_ALIEN_CURSOR_BIAS);
    xdb_i16 value;

    state->field_050 = (xdb_u16)(state->field_050 + 0x40u);
    if (state->field_040 <= 100u) {
        value = state->field_038;
        if (value <= 100 && value >= -100) {
            value = state->field_03c;
            if (value <= 100 && value >= -100) {
                xdb_alien_exit_requested = 1;
            }
        }
    }

    xdb_croolis_method_slot_6_wrap_positions(context);
}
