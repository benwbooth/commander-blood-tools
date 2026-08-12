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

cb_u16 CB_FAR bloodprg_strlen(const volatile char CB_FAR *text); /* 0x002665 */

#endif
