#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_fade_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 duration = (xdb_i16)(
            context->continuation.scrut_slot2.duration - 4);

    if (duration >= 0) {
        context->continuation.scrut_slot2.duration = duration;
        xdb_scrut_slot2_motion_update(state, context);
        return;
    }
    context->continuation.scrut_slot2.duration = 0;
    xdb_scrut_slot2_active = 0;
    xdb_scrut_slot2_restart(state, context);
}
