#ifndef BLOODPRG_HARDWARE_H
#define BLOODPRG_HARDWARE_H

#include "bloodprg_common.h"

typedef const cb_u8 CB_FAR *bloodprg_font_ptr;

extern volatile cb_u8 CB_GAME_DATA saved_video_mode; /* GS:0x5232 */
extern volatile cb_u16 cmos_seconds_pair;    /* CS:0x0AEE */
extern volatile cb_u16 CB_GAME_DATA video_crtc_base_port; /* GS:0x0A9E */
extern volatile cb_u16 video_crtc_base_port_ds; /* DS:0x0A9E alias */
extern volatile cb_u8 CB_GAME_DATA video_retrace_phase;   /* GS:0x0B12 */
extern volatile cb_u16 CB_GAME_DATA video_calibration_ticks; /* GS:0x0B35 */
extern bloodprg_font_ptr CB_GAME_DATA bios_font_8x8; /* GS:0x5225 */

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
void CB_FAR vga_retrace_phase_calibrate(void); /* 0x000B42 */
void CB_FAR video_retrace_phase_wait(void); /* 0x000BD7 */
void CB_FAR vga_mode_x_initialize(void);       /* 0x000C26 */
void CB_FAR set_video_mode_saved(void);      /* 0x000CC0 */
cb_u16 CB_FAR cpu_386_or_newer(void);          /* 0x000CCB */
void CB_FAR cmos_rtc_read(void);              /* 0x002DD3 */
void CB_FAR vga_palette_write(
        const volatile cb_u8 *palette);       /* 0x002F90 */
void CB_FAR vga_dac_clear(void);              /* 0x002FA6 */
cb_u16 CB_NEAR cb_flags_read(void);
void CB_NEAR cb_flags_write(cb_u16 flags);
cb_u32 CB_NEAR cb_bios_font_8x8_get(void);

#if defined(__WATCOMC__)
#pragma aux cb_flags_read = "pushf" "pop ax" value [ax] modify exact [ax]
#pragma aux cb_flags_write = "push ax" "popf" parm [ax] modify exact []
#pragma aux cb_bios_font_8x8_get = \
        "mov ax,1130h" "mov bh,3" "int 10h" "mov ax,bp" "mov dx,es" \
        value [dx ax] modify exact [ax bx dx es bp]
#pragma aux video_retrace_phase_wait modify exact []
#pragma aux vga_palette_write parm [si]
#endif

#endif
