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
extern void XDB_NEAR xdb_mouse_driver_set_vertical_bounds(
        xdb_u16 minimum, xdb_u16 maximum);
#pragma aux xdb_mouse_driver_set_vertical_bounds = \
        "mov ax,8" \
        "int 33h" \
        parm [cx] [dx] \
        modify exact [ax cx dx]
extern void XDB_NEAR xdb_mouse_driver_set_horizontal_bounds(
        xdb_u16 minimum, xdb_u16 maximum);
#pragma aux xdb_mouse_driver_set_horizontal_bounds = \
        "mov ax,7" \
        "int 33h" \
        parm [cx] [dx] \
        modify exact [ax cx dx]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define xdb_mouse_driver_set_position() \
    do { \
        _AX = 4; \
        geninterrupt(0x33); \
    } while (0)
#define xdb_mouse_driver_set_vertical_bounds(minimum, maximum) \
    do { \
        _AX = 8; \
        _CX = (minimum); \
        _DX = (maximum); \
        geninterrupt(0x33); \
    } while (0)
#define xdb_mouse_driver_set_horizontal_bounds(minimum, maximum) \
    do { \
        _AX = 7; \
        _CX = (minimum); \
        _DX = (maximum); \
        geninterrupt(0x33); \
    } while (0)
#else
void xdb_mouse_driver_set_position(void);
void xdb_mouse_driver_set_vertical_bounds(xdb_u16 minimum, xdb_u16 maximum);
void xdb_mouse_driver_set_horizontal_bounds(xdb_u16 minimum, xdb_u16 maximum);
#endif

void XDB_NEAR xdb_amer_mouse_position_set(xdb_u16 x, xdb_u16 y);
void XDB_NEAR xdb_croolis_mouse_position_set(xdb_u16 x, xdb_u16 y);
void XDB_NEAR xdb_scrut_mouse_position_set(xdb_u16 x, xdb_u16 y);
void XDB_NEAR xdb_amer_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y);
void XDB_NEAR xdb_croolis_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y);
void XDB_NEAR xdb_scrut_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y);

#if defined(__WATCOMC__)
#pragma aux xdb_amer_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_croolis_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_scrut_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_amer_mouse_bounds_set parm [cx] [dx] modify exact [ax cx dx]
#pragma aux xdb_croolis_mouse_bounds_set parm [cx] [dx] modify exact [ax cx dx]
#pragma aux xdb_scrut_mouse_bounds_set parm [cx] [dx] modify exact [ax cx dx]
#endif

#endif
