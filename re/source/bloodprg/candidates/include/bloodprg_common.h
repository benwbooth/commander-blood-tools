#ifndef BLOODPRG_COMMON_H
#define BLOODPRG_COMMON_H

typedef unsigned char cb_u8;
typedef unsigned int cb_u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define CB_FAR far
#define CB_NEAR near
#else
#define CB_FAR
#define CB_NEAR
#endif

#endif
