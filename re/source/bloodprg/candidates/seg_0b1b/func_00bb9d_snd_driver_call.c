#include "../include/bloodprg_audio.h"

void CB_FAR snd_driver_call(void)
{
    snd_driver_callback(0u);
    snd_driver_pending_flag = 0;
}
