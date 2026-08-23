#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_method_slot_13_resume_or_init(
    xdb_alien_method_context XDB_NEAR *context)
{
#if defined(__WATCOMC__)
    xdb_alien_resume_tail_or_continue();
#else
    xdb_alien_resume_callback resume = context->control.resume;

    if (resume != 0) {
        resume(context);
        return;
    }
#endif

    context->control.resume = xdb_amer_resume_1c34;
    context->continuation.resume_state.phase = 0;
    context->continuation.resume_state.paired_state = 0;
}
