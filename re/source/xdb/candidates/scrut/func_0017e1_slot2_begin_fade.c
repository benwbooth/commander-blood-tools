#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_begin_fade(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->callback = xdb_scrut_slot2_fade_update;
    xdb_scrut_slot2_fade_update(state, context);
}
