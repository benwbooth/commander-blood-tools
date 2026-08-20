#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_unreferenced_steering_update(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *state =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_i32 score;
    xdb_i16 turn;

    state->field_054 = 0x0a;
    score = (xdb_i32)state->field_038 * state->field_032
            - (xdb_i32)(xdb_i16)state->field_040 * state->field_01a;
    turn = score < 0 ? 0x10 : (xdb_i16)0xfff0u;
    if ((xdb_i16)(state->field_058 ^ (xdb_u16)turn) < 0) {
        turn = (xdb_i16)(turn >> 1);
    }
    state->field_058 = (xdb_u16)turn;
    state->field_050 = (xdb_u16)(state->field_050 + (xdb_u16)turn);
}
