#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_resume_stage_pair(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *current =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_biased_state XDB_NEAR *other =
            (xdb_alien_biased_state XDB_NEAR *)(size_t)
            context->continuation.resume_state.paired_state;
    xdb_i16 half_other = (xdb_i16)(
            ((xdb_u16)other->field_054 >> 1)
            | ((xdb_u16)other->field_054 & 0x8000u));
    xdb_i16 average;

    xdb_croolis_resume_apply_object_delta(context);
    average = (xdb_i16)((xdb_u16)other->field_054
            + (xdb_u16)current->field_054);
    average = (xdb_i16)((xdb_u16)average + (xdb_u16)half_other);
    current->field_054 = (xdb_i16)(
            ((xdb_u16)average >> 1) | ((xdb_u16)average & 0x8000u));
    if (xdb_croolis_resume_pair_outside(current, other)) {
        return;
    }

    context->control.resume = xdb_croolis_resume_stage_timeout;
    current->field_054 = 0;
    other->callback = xdb_croolis_slot3_resume_callback;
    context->continuation.resume_state.resumed_state = (xdb_u16)(size_t)other;
    xdb_croolis_slot3_resume_countdown = 0x18;
}
