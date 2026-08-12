#ifndef BLOODPRG_AUDIO_H
#define BLOODPRG_AUDIO_H

#include "bloodprg_common.h"

typedef void (CB_FAR *bloodprg_snd_driver_callback)(cb_u16 command);
typedef cb_u16 (CB_FAR *bloodprg_audio_position_callback)(void);

/* Original 0xBB9D switches DS to GS before using these ordinary globals. */
extern bloodprg_snd_driver_callback snd_driver_callback; /* DS=GS:0x0CDF */
extern bloodprg_audio_position_callback audio_position_callback; /* DS:0x0CF3 */
extern volatile cb_u8 snd_driver_pending_flag; /* DS=GS:0x0BA0 */
extern volatile cb_u8 voc_playback_enabled; /* game data:0x0ADE */

#if defined(__WATCOMC__)
#pragma aux snd_bank_loader parm [ax] [si] modify exact []
#endif

void CB_FAR snd_bank_loader(
    cb_u16 mode,
    const volatile char CB_NEAR *path); /* 0x0B1B:0855 */
void CB_FAR snd_driver_call(void);      /* 0x00BB9D */

#endif
