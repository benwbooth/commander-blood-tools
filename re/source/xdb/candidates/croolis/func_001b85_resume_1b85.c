#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_resume_1b85(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 queue_cursor = xdb_croolis_slot11_queue_read_cursor;
    xdb_u16 queued_state = xdb_croolis_slot11_state_queue[queue_cursor >> 1];

    if (queued_state == 0u) {
        queue_cursor = (xdb_u16)((queue_cursor + 2u) & 0x000fu);
        xdb_croolis_slot11_queue_read_cursor = queue_cursor;
        context->state->field_0ac = (xdb_i16)(
                ((xdb_u16)context->state->field_0ac - 0x07e0u
                 & 0x0ffcu) - 0x0800u);
        return;
    }

    xdb_croolis_slot11_current_state = 0;
    xdb_croolis_slot11_state_queue[queue_cursor >> 1] = 0;
    context->control.resume = xdb_croolis_resume_stage_pair;
    context->continuation.resume_state.paired_state = queued_state;
    xdb_croolis_resume_stage_pair(context);
}
