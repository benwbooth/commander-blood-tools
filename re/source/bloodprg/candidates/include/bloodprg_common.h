#ifndef BLOODPRG_COMMON_H
#define BLOODPRG_COMMON_H

typedef unsigned char cb_u8;
typedef unsigned int cb_u16;
typedef signed char cb_i8;
typedef signed int cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define CB_FAR far
#define CB_NEAR near
#else
#define CB_FAR
#define CB_NEAR
#endif

#if defined(__WATCOMC__)
#define CB_GAME_DATA __based(__segname("GAME_DATA"))
#else
#define CB_GAME_DATA CB_FAR
#endif

cb_u16 CB_FAR bloodprg_strlen(const volatile char CB_FAR *text); /* 0x002665 */
cb_u8 CB_NEAR bcd_to_binary(cb_u8 value);                        /* 0x000986 */
void CB_NEAR mem_copy_words(cb_u16 *dst, const cb_u16 *src);      /* 0x00A7E6 */

#if defined(__WATCOMC__)
#pragma aux bcd_to_binary parm [ax] value [al] modify [ax]
#endif

#endif
