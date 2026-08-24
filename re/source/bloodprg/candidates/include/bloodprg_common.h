#ifndef BLOODPRG_COMMON_H
#define BLOODPRG_COMMON_H

#include <stddef.h>

typedef unsigned char cb_u8;
typedef unsigned int cb_u16;
typedef signed char cb_i8;
typedef signed int cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__)
#define CB_OFFSETOF(type, member) \
    ((cb_u16)(unsigned)&(((type *)0)->member))
#else
#define CB_OFFSETOF(type, member) ((cb_u16)offsetof(type, member))
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define CB_FAR far
#define CB_NEAR near
#else
#define CB_FAR
#define CB_NEAR
#endif

#if defined(__WATCOMC__)
#define CB_INTERRUPT __interrupt
#define CB_SAVE_REGS __saveregs
#define CB_LOAD_DS __loadds
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define CB_INTERRUPT interrupt
#define CB_SAVE_REGS
#define CB_LOAD_DS
#else
#define CB_INTERRUPT
#define CB_SAVE_REGS
#define CB_LOAD_DS
#endif

#if defined(__WATCOMC__)
#define CB_CODE_DATA __based(__segname("_CODE"))
#define CB_GAME_DATA __based(__segname("GAME_DATA"))
#define CB_FS_DATA CB_FAR
#else
#define CB_CODE_DATA CB_FAR
#define CB_GAME_DATA CB_FAR
#define CB_FS_DATA CB_FAR
#endif

extern cb_u8 CB_CODE_DATA decimal_append_scratch[12];

cb_u16 CB_FAR bloodprg_strlen(const volatile char CB_FAR *text); /* 0x002665 */
cb_i16 CB_FAR ascii_digit_parse(const char CB_NEAR *text);       /* 0x002612 */
void CB_FAR decimal_append_i16(
        cb_i16 value, char CB_FAR *destination);                 /* 0x0024B2 */
void CB_FAR decimal_append_i32(
        cb_i32 value, char CB_FAR *destination);                 /* 0x0024EB */
cb_u8 CB_NEAR bcd_to_binary(cb_u8 value);                        /* 0x000986 */
void CB_NEAR mem_copy_words(cb_u16 *dst, const cb_u16 *src);      /* 0x00A7E6 */

#if defined(__WATCOMC__)
#pragma aux bloodprg_strlen parm [es di] value [ax] modify exact [ax]
#pragma aux ascii_digit_parse parm [si] value [ax] modify exact [ax]
#pragma aux decimal_append_i16 parm [ax] [es di] modify exact [ax es]
#pragma aux decimal_append_i32 parm [dx ax] [es di] modify exact [ax es]
#pragma aux bcd_to_binary parm [ax] value [al] modify [ax]
#pragma aux mem_copy_words parm [di] [si] modify exact [ax es di si]
#endif

#endif
