#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_method_slot_13_resume_or_init(
    xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_resume_callback resume;

    resume = context->resume;
    if (resume != 0) {
        resume(context);
        return;
    }

    context->resume = xdb_croolis_resume_1b85;
    context->resume_step = 0;
    context->resume_value = 0;
}
