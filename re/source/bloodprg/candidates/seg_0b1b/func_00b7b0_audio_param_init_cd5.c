#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_startup.h"

void CB_FAR audio_param_init_cd5(cb_u16 driver_segment)
{
    cb_u16 index;

    for (index = 0; index < 9u; ++index) {
        snd_driver_entries[index].address.segment = driver_segment;
    }

    snd_play_clip_callback = snd_play_clip;
    snd_driver_entries[0].initialize(startup_audio_configuration);
}
