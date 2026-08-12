#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_FAR mouse_set_ranges(cb_u16 min_x, cb_u16 max_x,
        cb_u16 min_y, cb_u16 max_y)
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
