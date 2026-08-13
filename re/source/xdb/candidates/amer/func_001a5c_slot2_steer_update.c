#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot2_steer_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u32 first_factor =
            (xdb_u32)(xdb_i32)(xdb_i16)state->field_040 -
            (xdb_u16)xdb_alien_camera_depth_step - 0x03e8UL;
    xdb_u32 score;
    xdb_u16 countdown;

    (void)context;
    first_factor = 0UL - first_factor;
    score = first_factor * (xdb_u32)state->field_01a;
    score += (xdb_u32)(xdb_i32)state->field_038 *
             (xdb_u32)state->field_032;
    state->field_050 += (xdb_i32)score < 0 ? 0x20u : 0xffe0u;

    countdown = (xdb_u16)state->field_056 - 1u;
    state->field_056 = (xdb_i16)countdown;
    if ((countdown & 0x8000u) != 0) {
        state->callback = xdb_amer_slot2_finish_update;
        state->field_056 = 0x40;
    }
}
