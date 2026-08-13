/*
 * Codegen probe for BLOODPRG 0x00BC50.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#define FP_OFF(pointer) ((u16)(pointer))
#endif

typedef struct snd_stream_buffer {
    volatile u8 FAR *data;
    u16 byte_count;
    u8 state;
    u8 reserved_07;
} snd_stream_buffer;

typedef u16 (FAR *audio_position_callback_type)(void);

extern volatile u8 voc_playback_enabled;
extern volatile u8 snd_driver_pending_flag;
extern volatile u8 snd_stream_channel_active;
extern volatile u16 snd_stream_header[3];
extern volatile u16 snd_stream_next_page;
extern volatile u16 snd_stream_page_count;
extern volatile u16 snd_stream_final_page_bytes;
extern volatile u8 FAR *snd_stream_storage;
extern volatile snd_stream_buffer snd_stream_buffers[2];
extern audio_position_callback_type audio_position_callback;

void NEAR snd_bank_page_read_probe(u16 page, volatile u8 FAR *destination);
void NEAR cb_snd_stream_service_probe(u16 command,
        volatile snd_stream_buffer *buffer,
        volatile u8 FAR *cursor);
void NEAR cb_snd_stream_play_probe(u16 command,
        volatile snd_stream_buffer *buffer,
        volatile u8 FAR *cursor);

#if defined(__WATCOMC__)
#pragma aux snd_bank_page_read_probe parm [ax] [es di] modify exact []
#pragma aux cb_snd_stream_service_probe parm [ax] [si] [es di]
#pragma aux cb_snd_stream_play_probe parm [ax] [si] [es di]
#endif

void FAR snd_stream_refill_probe(void)
{
    volatile snd_stream_buffer *buffer;
    volatile u8 FAR *cursor;
    u16 page;
    u16 position;

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
            ((volatile u16 FAR *)cursor)[0] = snd_stream_header[0];
            ((volatile u16 FAR *)cursor)[1] = snd_stream_header[1];
            ((volatile u16 FAR *)cursor)[2] = snd_stream_header[2];
            cursor += 6;
        }

        snd_bank_page_read_probe(page, cursor);
        buffer->state = 1u;
        buffer->byte_count = 0x4000u;
        ++page;
        if (page >= snd_stream_page_count) {
            buffer->byte_count = snd_stream_final_page_bytes;
            page = 0;
        }
        snd_stream_next_page = page;

        if (position != 0 && position != 0xFFFFu) {
            cb_snd_stream_service_probe(0u, buffer, cursor);
        } else {
            snd_stream_buffers[0].state =
                    (u8)(buffer == &snd_stream_buffers[0]);
            snd_stream_buffers[1].state =
                    (u8)(buffer != &snd_stream_buffers[0]);
            if (FP_OFF(cursor) != FP_OFF(snd_stream_storage)) {
                cursor -= 6;
            }
            cb_snd_stream_play_probe(0u, buffer, cursor);
        }
    }
}
