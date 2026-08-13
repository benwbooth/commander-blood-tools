#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_method_slot_13_resume_or_init(
    xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_resume_callback resume;

    resume = context->control.resume;
    if (resume != 0) {
        resume(context);
        return;
    }

    context->control.resume = xdb_croolis_resume_1b85;
    context->continuation.resume_state.step = 0;
    context->continuation.resume_state.value = 0;
}
