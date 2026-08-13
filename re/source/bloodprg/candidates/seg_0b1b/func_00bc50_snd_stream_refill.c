#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"

void CB_FAR snd_stream_refill(void)
{
    volatile bloodprg_snd_stream_buffer *buffer;
    volatile cb_u8 CB_FAR *cursor;
    cb_u16 page;
    cb_u16 position;

    if ((voc_playback_enabled & 1u) == 0
            || (snd_stream_channel_active & 1u) == 0
            || (snd_driver_pending_flag & 2u) == 0) {
        return;
    }

    for (;;) {
        position = audio_position_callback();
        buffer = &snd_stream_buffers[0];
        if ((buffer->state & 2u) != 0) {
            ++buffer;
            if ((buffer->state & 2u) != 0
                    && position != 0
                    && position != 0xFFFFu) {
                return;
            }
        }

        cursor = buffer->data;
        page = snd_stream_next_page;
        if (page != 0) {
            ((volatile cb_u16 CB_FAR *)cursor)[0] = snd_stream_header[0];
            ((volatile cb_u16 CB_FAR *)cursor)[1] = snd_stream_header[1];
            ((volatile cb_u16 CB_FAR *)cursor)[2] = snd_stream_header[2];
            cursor += 6;
        }

        snd_bank_page_read(page, cursor);
        buffer->state = 1u;
        buffer->byte_count = 0x4000u;
        ++page;
        if (page >= snd_stream_page_count) {
            buffer->byte_count = snd_stream_final_page_bytes;
            page = 0;
        }
        snd_stream_next_page = page;

        if (position != 0 && position != 0xFFFFu) {
            cb_snd_stream_service(0u, buffer, cursor);
        } else {
            snd_stream_buffers[0].state =
                    (cb_u8)(buffer == &snd_stream_buffers[0]);
            snd_stream_buffers[1].state =
                    (cb_u8)(buffer != &snd_stream_buffers[0]);
            if (FP_OFF(cursor) != FP_OFF(snd_stream_storage)) {
                cursor -= 6;
            }
            cb_snd_stream_play(0u, buffer, cursor);
        }
    }
}
