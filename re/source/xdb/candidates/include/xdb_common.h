#ifndef XDB_COMMON_H
#define XDB_COMMON_H

typedef unsigned char xdb_u8;
typedef unsigned int xdb_u16;
typedef signed char xdb_i8;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;
typedef signed long xdb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define XDB_FAR far
#define XDB_NEAR near
#define XDB_FAR_AT(type, segment, offset) \
    ((type XDB_FAR *)MK_FP((segment), (offset)))
#else
#define XDB_FAR
#define XDB_NEAR
#endif

#if defined(__WATCOMC__)
#define XDB_CODE_DATA __based(__segname("_CODE"))
#else
#define XDB_CODE_DATA XDB_FAR
#endif

/* All recovered C entry points follow the compiler's clear-direction ABI. */

#endif
