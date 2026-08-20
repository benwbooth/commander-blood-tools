#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_resume_stage_final(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *current =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_biased_state XDB_NEAR *other =
            (xdb_alien_biased_state XDB_NEAR *)(size_t)xdb_croolis_slot11_cursor;

    current->field_054 = 0x64;
    if (xdb_croolis_resume_pair_outside(current, other)) {
        return;
    }
    context->control.resume = xdb_croolis_resume_1b85;
    other = (xdb_alien_biased_state XDB_NEAR *)(size_t)
            context->continuation.resume_state.paired_state;
    other->callback = xdb_croolis_slot3_restart_initial_update;
    current->field_054 = 0;
}
