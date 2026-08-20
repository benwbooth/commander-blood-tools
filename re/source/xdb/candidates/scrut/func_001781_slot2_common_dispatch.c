#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_common_dispatch(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    if (xdb_alien_control_latch == (xdb_u16)(size_t)context) {
        return;
    }
    xdb_scrut_slot2_motion_update(state, context);
}
