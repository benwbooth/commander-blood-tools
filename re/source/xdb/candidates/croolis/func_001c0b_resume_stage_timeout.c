#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_resume_stage_timeout(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_croolis_resume_apply_object_delta(context);
    --xdb_croolis_slot3_resume_countdown;
    if ((xdb_i16)xdb_croolis_slot3_resume_countdown < 0) {
        context->control.resume = xdb_croolis_resume_stage_final;
    }
}
