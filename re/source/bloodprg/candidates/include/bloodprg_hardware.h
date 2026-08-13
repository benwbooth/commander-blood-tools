#ifndef BLOODPRG_HARDWARE_H
#define BLOODPRG_HARDWARE_H

#include "bloodprg_common.h"

extern volatile cb_u8 saved_video_mode;      /* GS:0x5232 */
extern volatile cb_u16 cmos_seconds_pair;    /* CS:0x0AEE */

typedef void (CB_INTERRUPT CB_FAR *bloodprg_interrupt_handler)(void);

extern bloodprg_interrupt_handler CB_GAME_DATA
        timer_previous_handler; /* GS:0x0B1D */
extern volatile cb_u8 CB_GAME_DATA timer_hook_active; /* GS:0x0B21 */
extern volatile cb_u8 CB_GAME_DATA timer_divider;     /* GS:0x0B22 */
extern volatile cb_u16 CB_GAME_DATA timer_reload_ticks; /* GS:0x0B25 */
extern volatile cb_u16 CB_GAME_DATA timer_subtick_limit; /* GS:0x0B27 */

void CB_INTERRUPT CB_FAR bloodprg_timer_isr(void);          /* CS:0x0213 */
void CB_INTERRUPT CB_FAR bloodprg_ctrl_break_handler(void); /* CS:0x0619 */
void CB_INTERRUPT CB_FAR bloodprg_critical_error_handler(void); /* CS:0x061A */

void CB_FAR install_timer_isr_hook(void); /* 0x00079C */
void CB_FAR restore_timer_isr_hook(void); /* 0x0007EA */
void CB_FAR install_ctrl_break_handler(void); /* 0x000BFF */
void CB_FAR set_video_mode_saved(void);      /* 0x000CC0 */
void CB_FAR cmos_rtc_read(void);              /* 0x002DD3 */
void CB_FAR vga_palette_write(
        const volatile cb_u8 *palette);       /* 0x002F90 */
void CB_FAR vga_dac_clear(void);              /* 0x002FA6 */

#if defined(__WATCOMC__)
#pragma aux vga_palette_write parm [si]
#endif

#endif
