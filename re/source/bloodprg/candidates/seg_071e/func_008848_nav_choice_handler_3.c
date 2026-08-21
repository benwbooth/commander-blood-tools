#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

void CB_NEAR nav_choice_handler_3(void)
{
    if ((nav_choice_phase & 1u) == 0) {
        return;
    }

    nav_deferred_record_link = nav_choice_radio_record;
    nav_deferred_record_type = 0x00c3u;
    nav_choice_phase = 0;
    snd_bank_loader(1u, nav_radio_snd_path);
}
