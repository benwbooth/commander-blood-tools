#ifndef XDB_MOUSE_H
#define XDB_MOUSE_H

#include "xdb_common.h"

typedef struct xdb_mouse_state {
    xdb_u16 x;
    xdb_u16 y;
    xdb_u16 buttons;
} xdb_mouse_state;

extern volatile xdb_mouse_state xdb_alien_mouse_state; /* DS:0x002A */
#define xdb_alien_mouse_x xdb_alien_mouse_state.x
#define xdb_alien_mouse_y xdb_alien_mouse_state.y
#define xdb_alien_mouse_buttons xdb_alien_mouse_state.buttons

#if defined(__WATCOMC__)
#define xdb_mouse_driver_poll() \
    do { \
        union REGS registers; \
        registers.x.ax = 3u; \
        registers.x.bx = 0u; \
        registers.x.cx = 0u; \
        registers.x.dx = 0u; \
        registers.x.si = 0u; \
        registers.x.di = 0u; \
        int86(0x33, &registers, &registers); \
        xdb_alien_mouse_x = registers.x.cx; \
        xdb_alien_mouse_y = registers.x.dx; \
        xdb_alien_mouse_buttons = registers.x.bx; \
    } while (0)
#define xdb_mouse_driver_command(function, position_x, position_y) \
    do { \
        union REGS registers; \
        registers.x.ax = (function); \
        registers.x.bx = 0u; \
        registers.x.cx = (position_x); \
        registers.x.dx = (position_y); \
        registers.x.si = 0u; \
        registers.x.di = 0u; \
        int86(0x33, &registers, &registers); \
    } while (0)
#define xdb_mouse_driver_set_position(x, y) \
    xdb_mouse_driver_command(4, (x), (y))
#define xdb_mouse_driver_set_vertical_bounds(minimum, maximum) \
    xdb_mouse_driver_command(8, (minimum), (maximum))
#define xdb_mouse_driver_set_horizontal_bounds(minimum, maximum) \
    xdb_mouse_driver_command(7, (minimum), (maximum))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define xdb_mouse_driver_poll() \
    do { \
        _AX = 3; \
        geninterrupt(0x33); \
        xdb_alien_mouse_x = _CX; \
        xdb_alien_mouse_y = _DX; \
        xdb_alien_mouse_buttons = _BX; \
    } while (0)
#define xdb_mouse_driver_set_position(x, y) \
    do { \
        _AX = 4; \
        _CX = (x); \
        _DX = (y); \
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
void xdb_mouse_driver_poll(void);
void xdb_mouse_driver_set_position(xdb_u16 x, xdb_u16 y);
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
