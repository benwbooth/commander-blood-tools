#ifndef XDB_MOUSE_H
#define XDB_MOUSE_H

#include "xdb_common.h"

extern volatile xdb_u16 xdb_alien_mouse_x; /* DS:0x002A */
extern volatile xdb_u16 xdb_alien_mouse_y; /* DS:0x002C */

#if defined(__WATCOMC__)
extern void XDB_NEAR xdb_mouse_driver_set_position(void);
#pragma aux xdb_mouse_driver_set_position = \
        "mov ax,4" \
        "int 33h" \
        modify exact [ax]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define xdb_mouse_driver_set_position() \
    do { \
        _AX = 4; \
        geninterrupt(0x33); \
    } while (0)
#else
void xdb_mouse_driver_set_position(void);
#endif

void XDB_NEAR xdb_amer_mouse_position_set(xdb_u16 x, xdb_u16 y);
void XDB_NEAR xdb_croolis_mouse_position_set(xdb_u16 x, xdb_u16 y);
void XDB_NEAR xdb_scrut_mouse_position_set(xdb_u16 x, xdb_u16 y);

#if defined(__WATCOMC__)
#pragma aux xdb_amer_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_croolis_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_scrut_mouse_position_set parm [cx] [dx] modify exact [ax]
#endif

#endif
