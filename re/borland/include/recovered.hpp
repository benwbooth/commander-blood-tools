#ifndef CB_RECOVERED_HPP
#define CB_RECOVERED_HPP

#if defined(__BORLANDC__)
#define CB_NEAR near
#define CB_FAR far
#else
#define CB_NEAR
#define CB_FAR
#endif

typedef unsigned char cb_u8;
typedef signed char cb_i8;
typedef unsigned short cb_u16;
typedef signed short cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

#endif
