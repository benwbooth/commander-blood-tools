#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_method_slot_13_resume_or_init(
    xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_resume_callback resume;

    resume = context->resume;
    if (resume != 0) {
        resume(context);
        return;
    }

    context->resume = xdb_scrut_resume_1c45;
    context->continuation.resume_state.step = 0;
    context->continuation.resume_state.value = 0;
}
