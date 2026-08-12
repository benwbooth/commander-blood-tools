#include "../include/bloodprg_audio.h"

void CB_FAR snd_driver_call(void)
{
    snd_driver_callback();
    snd_driver_pending_flag = 0;
}
