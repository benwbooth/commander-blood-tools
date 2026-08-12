#ifndef BLOODPRG_HARDWARE_H
#define BLOODPRG_HARDWARE_H

#include "bloodprg_common.h"

extern volatile cb_u8 saved_video_mode;      /* GS:0x5232 */
extern volatile cb_u16 cmos_seconds_pair;    /* CS:0x0AEE */

void CB_FAR set_video_mode_saved(void);      /* 0x000CC0 */
void CB_FAR cmos_rtc_read(void);              /* 0x002DD3 */
void CB_FAR vga_palette_write(
        const volatile cb_u8 *palette);       /* 0x002F90 */
void CB_FAR vga_dac_clear(void);              /* 0x002FA6 */

#if defined(__WATCOMC__)
#pragma aux vga_palette_write parm [si]
#endif

#endif
