#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"

void CB_FAR snd_stream_start(void)
{
    volatile cb_u8 CB_FAR *first_page;

    if ((voc_playback_enabled_gs & 1u) == 0
            || (snd_stream_channel_active & 1u) == 0
            || (snd_driver_pending_flag_gs & 3u) == 0) {
        return;
    }

    snd_stream_header_mode = 0;
    first_page = snd_stream_storage;
    snd_stream_buffers[0].data = first_page;
    snd_stream_buffers[0].byte_count = 0x4000u;
    snd_bank_page_read(0u, first_page);
    snd_stream_next_page = 1u;

    if (first_page[4] == 0xD3u) {
        snd_stream_header_mode = 1u;
    }
    snd_stream_header[0] = ((volatile cb_u16 CB_FAR *)first_page)[0];
    snd_stream_header[1] = ((volatile cb_u16 CB_FAR *)first_page)[1];
    snd_stream_header[2] = ((volatile cb_u16 CB_FAR *)first_page)[2];

    snd_stream_buffers[1].data = first_page + 0x4008u;
    snd_stream_buffers[1].byte_count = 0x4000u;
    snd_stream_buffers[1].state = 0;

    snd_driver_call();
    snd_driver_pending_flag_gs = 2u;
    snd_stream_buffers[0].state = 1u;
    cb_snd_stream_play(0u, &snd_stream_buffers[0], first_page + 0x4008u);
}
