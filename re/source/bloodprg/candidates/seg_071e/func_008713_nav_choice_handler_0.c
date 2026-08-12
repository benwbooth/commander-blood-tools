#include "../include/bloodprg_nav.h"

void CB_NEAR nav_choice_handler_0(void)
{
    if ((nav_choice_phase & 1u) != 0) {
        nav_deferred_record_link = nav_choice_honk_record;
        nav_deferred_record_type = 0x00c3u;
        nav_choice_phase = 0;
    }
}
