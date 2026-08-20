#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot3_capture_resume_state(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    xdb_amer_slot3_resume_countdown = 0x12;
    xdb_amer_slot3_resume_state = (xdb_alien_cursor)state;
    state->position_x = 0;
    state->position_y = 0x06a4L;
    state->position_z = 0;
    state->field_04e = 0;
    state->field_050 = 0;
    state->field_052 = 0;
    state->field_054 = 0;
}
