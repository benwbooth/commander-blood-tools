#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_FAR mouse_reset_hide(void)
{
    union REGS registers;

    registers.x.ax = 0u;
    int86(0x33, &registers, &registers);

    registers.x.ax = 2u;
    int86(0x33, &registers, &registers);

    registers.x.ax = 0x000fu;
    registers.x.cx = 0x000cu;
    registers.x.dx = 0x000cu;
    int86(0x33, &registers, &registers);
}
