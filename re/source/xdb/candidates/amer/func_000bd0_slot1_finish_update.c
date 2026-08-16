#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot1_finish_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 low_word;

    (void)context;
    low_word = (xdb_u16)(xdb_amer_slot1_current_sample - 0x00b0);
    state->position_y = (xdb_i32)(
            ((xdb_u32)state->position_y & 0xffff0000UL) | low_word);
    state->field_04e = (xdb_i16)(state->field_04e + 0x00a0);
    state->field_050 = (xdb_u16)(state->field_050 + 0x00d0);
    state->field_052 = (xdb_i16)(state->field_052 + 0x00e0);
}
