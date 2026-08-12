#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_NEAR detect_cdrom(void)
{
    union REGS registers;

    registers.x.ax = 0x1500;
    registers.x.bx = 0;
    int86(0x2F, &registers, &registers);
    cdrom_present = (cb_u8)(registers.x.bx != 0);
}
