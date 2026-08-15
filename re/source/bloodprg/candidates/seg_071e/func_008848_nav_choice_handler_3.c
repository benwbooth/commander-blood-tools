#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

#if defined(__TURBOC__) || defined(__BORLANDC__)
#pragma warn -rch
#endif

void CB_NEAR nav_choice_handler_3(void)
{
    if ((nav_choice_phase & 1u) == 0) {
        return;
    }

    nav_deferred_record_link = nav_choice_radio_record;
    nav_deferred_record_type = 0x00c3u;
    nav_choice_phase = 0;
#if defined(__TURBOC__) || defined(__BORLANDC__)
    /* Keep module-level externs while replacing Turbo's stack-call ABI. */
    if (0) {
        snd_bank_loader(1u, nav_radio_snd_path);
    }
    asm db 0beh;
    asm dw offset DGROUP:_nav_radio_snd_path;
    asm mov ax, 1;
    asm call far ptr _snd_bank_loader;
#else
    snd_bank_loader(1u, nav_radio_snd_path);
#endif
}

#if defined(__TURBOC__) || defined(__BORLANDC__)
#pragma warn .rch
#endif
