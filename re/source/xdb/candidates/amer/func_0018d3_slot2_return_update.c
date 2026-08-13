#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot2_return_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 countdown =
            (xdb_u16)context->continuation.amer_slot2_motion.countdown - 1u;
    xdb_i16 horizontal;
    xdb_i16 vertical;

    context->continuation.amer_slot2_motion.countdown = (xdb_i16)countdown;
    state->field_054 = 0;
    if ((countdown & 0x8000u) == 0) {
        state->field_050 += 0x80u;
        state->field_052 = (xdb_i16)((xdb_u16)state->field_052 - 0x75u);
        state->position_x = (xdb_i32)(
                (xdb_u32)state->position_x +
                context->continuation.amer_slot2_motion.velocity_x);
        state->position_y = (xdb_i32)(
                (xdb_u32)state->position_y +
                context->continuation.amer_slot2_motion.velocity_y);
        state->position_z = (xdb_i32)(
                (xdb_u32)state->position_z +
                context->continuation.amer_slot2_motion.velocity_z);
        return;
    }

    context->control.state = 1;
    context->continuation.amer_slot2_motion.countdown = 0x20;
    horizontal = (xdb_i16)(xdb_u16)(state->field_050 << 4);
    horizontal >>= 4;
    vertical = (xdb_i16)(xdb_u16)((xdb_u16)state->field_052 << 4);
    vertical >>= 4;
    state->field_050 = (xdb_u16)horizontal;
    state->field_052 = vertical;
    context->continuation.amer_slot2_motion.velocity_x =
            (xdb_i16)(0u - (xdb_u16)vertical);
    context->continuation.amer_slot2_motion.velocity_x >>= 5;
    state->callback = xdb_amer_slot2_update;
    xdb_amer_slot2_active = 0;
}
