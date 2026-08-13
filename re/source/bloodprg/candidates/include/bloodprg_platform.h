#ifndef BLOODPRG_PLATFORM_H
#define BLOODPRG_PLATFORM_H

#include "bloodprg_common.h"

extern volatile cb_i16 rtc_hour;        /* GS:0x0AA6 */
extern volatile cb_u8 cdrom_present;    /* GS:0x0AE6 */

void CB_FAR rtc_time_read(void);        /* 0x00093B */
void CB_NEAR detect_cdrom(void);        /* 0x000B32 */
void CB_FAR mouse_set_ranges(cb_u16 min_x, cb_u16 max_x,
        cb_u16 min_y, cb_u16 max_y);    /* 0x000D4A */
void CB_FAR mouse_reset_hide(void);      /* 0x000CEF */
void CB_FAR print_string_dos(
        const volatile char *text);     /* 0x000D61 */
cb_u16 CB_FAR kbd_read_int16(void);     /* 0x00267D */

#if defined(__WATCOMC__)
#pragma aux mouse_set_ranges parm [ax] [bx] [cx] [dx]
#pragma aux print_string_dos parm [si]
#endif

#endif
