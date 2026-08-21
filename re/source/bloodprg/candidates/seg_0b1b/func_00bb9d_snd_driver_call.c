#include "../include/bloodprg_audio.h"

void CB_FAR snd_driver_call(void)
{
#if defined(__WATCOMC__)
    _asm push ds;
    _asm push es;
#endif
    snd_driver_callback(0u);
#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ds;
#endif
    snd_driver_pending_flag = 0;
}
