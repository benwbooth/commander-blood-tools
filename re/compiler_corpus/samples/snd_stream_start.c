/*
 * Codegen probe for BLOODPRG 0x00BBB3.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct snd_stream_buffer {
    volatile u8 FAR *data;
    u16 byte_count;
    u8 state;
    u8 reserved_07;
} snd_stream_buffer;

extern volatile u8 voc_playback_enabled;
extern volatile u8 snd_driver_pending_flag;
extern volatile u8 snd_stream_channel_active;
extern volatile u8 snd_stream_header_mode;
extern volatile u16 snd_stream_header[3];
extern volatile u16 snd_stream_next_page;
extern volatile u8 FAR *snd_stream_storage;
extern volatile snd_stream_buffer snd_stream_buffers[2];

void NEAR snd_bank_page_read_probe(u16 page, volatile u8 FAR *destination);
void FAR snd_driver_call_probe(void);
void NEAR cb_snd_stream_play_probe(u16 command,
        volatile snd_stream_buffer *buffer,
        volatile u8 FAR *cursor);

#if defined(__WATCOMC__)
#pragma aux snd_bank_page_read_probe parm [ax] [es di] modify exact []
#pragma aux cb_snd_stream_play_probe parm [ax] [si] [es di]
#endif

void FAR snd_stream_start_probe(void)
{
    volatile u8 FAR *first_page;

    if ((voc_playback_enabled & 1u) == 0
            || (snd_stream_channel_active & 1u) == 0
            || (snd_driver_pending_flag & 3u) == 0) {
        return;
    }

    snd_stream_header_mode = 0;
    first_page = snd_stream_storage;
    snd_stream_buffers[0].data = first_page;
    snd_stream_buffers[0].byte_count = 0x4000u;
    snd_bank_page_read_probe(0u, first_page);
    snd_stream_next_page = 1u;

    if (first_page[4] == 0xD3u) {
        snd_stream_header_mode = 1u;
    }
    snd_stream_header[0] = ((volatile u16 FAR *)first_page)[0];
    snd_stream_header[1] = ((volatile u16 FAR *)first_page)[1];
    snd_stream_header[2] = ((volatile u16 FAR *)first_page)[2];

    snd_stream_buffers[1].data = first_page + 0x4008u;
    snd_stream_buffers[1].byte_count = 0x4000u;
    snd_stream_buffers[1].state = 0;

    snd_driver_call_probe();
    snd_driver_pending_flag = 2u;
    snd_stream_buffers[0].state = 1u;
    cb_snd_stream_play_probe(0u, &snd_stream_buffers[0], first_page + 0x4008u);
}
