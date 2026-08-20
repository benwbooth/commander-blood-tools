#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_resume_stage_timeout(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_amer_resume_apply_object_delta(context);
    --xdb_amer_slot3_resume_countdown;
    if ((xdb_i16)xdb_amer_slot3_resume_countdown < 0) {
        context->control.resume = xdb_amer_resume_stage_final;
    }
}
