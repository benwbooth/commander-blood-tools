/* Codegen probe for BLOODPRG 0x000D4A. */

#include <dos.h>

typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#pragma aux mouse_set_ranges_probe parm [ax] [bx] [cx] [dx]
#endif

void FAR mouse_set_ranges_probe(u16 min_x, u16 max_x,
        u16 min_y, u16 max_y)
{
    union REGS registers;

    registers.x.ax = 7;
    registers.x.bx = max_x;
    registers.x.cx = min_x;
    registers.x.dx = max_x;
    int86(0x33, &registers, &registers);

    registers.x.ax = 8;
    registers.x.cx = min_y;
    registers.x.dx = max_y;
    int86(0x33, &registers, &registers);
}
