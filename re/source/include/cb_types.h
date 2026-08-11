#ifndef CB_TYPES_H
#define CB_TYPES_H

#if defined(__BORLANDC__) || defined(__TURBOC__)
#define CB_FAR far
#define CB_NEAR near
#else
#define CB_FAR
#define CB_NEAR
#endif
typedef unsigned char cb_u8;
typedef signed char cb_i8;
typedef unsigned short cb_u16;
typedef signed short cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

#endif
