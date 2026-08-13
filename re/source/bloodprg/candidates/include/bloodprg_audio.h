#ifndef BLOODPRG_AUDIO_H
#define BLOODPRG_AUDIO_H

#include "bloodprg_common.h"

typedef void (CB_FAR *bloodprg_snd_driver_callback)(cb_u16 command);
typedef cb_u16 (CB_FAR *bloodprg_audio_position_callback)(void);

typedef struct bloodprg_snd_stream_buffer {
    volatile cb_u8 CB_FAR *data;
    cb_u16 byte_count;
    cb_u8 state;
    cb_u8 reserved_07;
} bloodprg_snd_stream_buffer;

typedef struct bloodprg_snd_bank_header {
    cb_u16 clip_count;
    cb_u16 reserved_02;
} bloodprg_snd_bank_header;

typedef struct bloodprg_snd_memory_clip {
    cb_u16 offset;
    cb_u16 byte_count;
} bloodprg_snd_memory_clip;

/* Original 0xBB9D switches DS to GS before using these ordinary globals. */
extern bloodprg_snd_driver_callback snd_driver_callback; /* DS=GS:0x0CDF */
extern bloodprg_audio_position_callback audio_position_callback; /* DS:0x0CF3 */
extern volatile cb_u8 snd_driver_pending_flag; /* DS=GS:0x0BA0 */
extern volatile cb_u8 voc_playback_enabled; /* game data:0x0ADE */
extern volatile bloodprg_snd_stream_buffer CB_GAME_DATA
        snd_stream_buffers[2]; /* GS:0x0B89 */
extern volatile cb_u16 CB_GAME_DATA snd_stream_header[3]; /* GS:0x0B99 */
extern volatile cb_u8 CB_GAME_DATA snd_stream_header_mode; /* GS:0x0BA2 */
extern volatile cb_u8 CB_GAME_DATA snd_stream_channel_active; /* GS:0x0BA3 */
extern volatile cb_u16 CB_GAME_DATA snd_stream_next_page; /* GS:0x0BA5 */
extern volatile cb_u16 CB_GAME_DATA snd_stream_page_count; /* GS:0x0BA7 */
extern volatile cb_u16 CB_GAME_DATA snd_stream_final_page_bytes; /* GS:0x0BA9 */
extern volatile cb_u8 CB_FAR *CB_GAME_DATA snd_stream_storage; /* GS:0x0BB7 */
extern volatile cb_u32 CB_GAME_DATA snd_source_remaining; /* GS:0x0A92 */
extern const volatile char CB_GAME_DATA snd_wait_prompt_text[]; /* GS:0x0190 */
extern const volatile char CB_GAME_DATA snd_music_temp_filename[]; /* GS:0x00AE */
extern const volatile char CB_GAME_DATA snd_voice_temp_filename[]; /* GS:0x00A6 */
extern volatile cb_u8 CB_FAR *CB_GAME_DATA snd_bank_memory; /* GS:0x0BB3 */
extern volatile bloodprg_snd_bank_header CB_GAME_DATA snd_bank_header; /* GS:0x0BBB */
extern volatile bloodprg_snd_memory_clip CB_GAME_DATA
        snd_memory_clips[]; /* GS:0x0BBF */
extern volatile cb_u32 CB_GAME_DATA snd_source_offsets[]; /* GS:0x0F1A */
extern volatile cb_u16 CB_GAME_DATA snd_streamed_clip_count; /* GS:0x0C53 */
extern volatile cb_u32 CB_GAME_DATA snd_streamed_offsets[]; /* GS:0x0C57 */

void CB_NEAR cb_snd_stream_service(cb_u16 command,
        volatile bloodprg_snd_stream_buffer *buffer,
        volatile cb_u8 CB_FAR *cursor);
void CB_NEAR cb_snd_stream_play(cb_u16 command,
        volatile bloodprg_snd_stream_buffer *buffer,
        volatile cb_u8 CB_FAR *cursor);

#if defined(__WATCOMC__)
#pragma aux snd_bank_loader parm [ax] [si] modify exact []
#pragma aux snd_stream_source_load parm [si] modify exact []
#pragma aux cb_snd_stream_service parm [ax] [si] [es di]
#pragma aux cb_snd_stream_play parm [ax] [si] [es di]
#endif

void CB_FAR snd_bank_loader(
    cb_u16 mode,
    const volatile char CB_NEAR *path); /* 0x0B1B:0855 */
void CB_FAR snd_driver_call(void);      /* 0x00BB9D */
void CB_FAR snd_stream_start(void);     /* 0x00BBB3 */
void CB_FAR snd_stream_refill(void);    /* 0x00BC50 */
void CB_FAR snd_stream_source_load(
        const volatile char CB_NEAR *path); /* 0x00BDB7 */

#endif
