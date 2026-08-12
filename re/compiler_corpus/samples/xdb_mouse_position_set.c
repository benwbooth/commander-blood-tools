/*
 * Codegen probe for the shared XDB mouse-position helper.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 mouse_x;
extern volatile u16 mouse_y;

#if defined(__WATCOMC__)
extern void NEAR mouse_driver_set_position(void);
#pragma aux mouse_driver_set_position = \
        "mov ax,4" \
        "int 33h" \
        modify exact [ax]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define mouse_driver_set_position() \
    do { \
        _AX = 4; \
        geninterrupt(0x33); \
    } while (0)
#else
void mouse_driver_set_position(void);
#endif

void NEAR xdb_mouse_position_set_probe(u16 x, u16 y);

#if defined(__WATCOMC__)
#pragma aux xdb_mouse_position_set_probe parm [cx] [dx] modify exact [ax]
#endif

void NEAR xdb_mouse_position_set_probe(u16 x, u16 y)
{
    mouse_x = x;
    mouse_y = y;
    mouse_driver_set_position();
}
