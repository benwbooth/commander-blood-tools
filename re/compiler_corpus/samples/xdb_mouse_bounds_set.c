/*
 * Codegen probe for the shared XDB mouse-bounds helper.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#if defined(__WATCOMC__)
extern void NEAR mouse_driver_set_vertical_bounds(u16 minimum, u16 maximum);
#pragma aux mouse_driver_set_vertical_bounds = \
        "mov ax,8" \
        "int 33h" \
        parm [cx] [dx] \
        modify exact [ax cx dx]
extern void NEAR mouse_driver_set_horizontal_bounds(u16 minimum, u16 maximum);
#pragma aux mouse_driver_set_horizontal_bounds = \
        "mov ax,7" \
        "int 33h" \
        parm [cx] [dx] \
        modify exact [ax cx dx]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define mouse_driver_set_vertical_bounds(minimum, maximum) \
    do { \
        _AX = 8; \
        _CX = (minimum); \
        _DX = (maximum); \
        geninterrupt(0x33); \
    } while (0)
#define mouse_driver_set_horizontal_bounds(minimum, maximum) \
    do { \
        _AX = 7; \
        _CX = (minimum); \
        _DX = (maximum); \
        geninterrupt(0x33); \
    } while (0)
#else
void mouse_driver_set_vertical_bounds(u16 minimum, u16 maximum);
void mouse_driver_set_horizontal_bounds(u16 minimum, u16 maximum);
#endif

void NEAR xdb_mouse_bounds_set_probe(u16 max_x, u16 max_y);

#if defined(__WATCOMC__)
#pragma aux xdb_mouse_bounds_set_probe parm [cx] [dx] modify exact [ax cx dx]
#endif

void NEAR xdb_mouse_bounds_set_probe(u16 max_x, u16 max_y)
{
    mouse_driver_set_vertical_bounds(0, max_y);
    mouse_driver_set_horizontal_bounds(0, max_x);
}
