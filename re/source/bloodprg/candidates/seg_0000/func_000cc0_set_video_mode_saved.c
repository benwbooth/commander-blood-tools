#include <dos.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR set_video_mode_saved(void)
{
    union REGS registers;

    registers.h.ah = 0;
    registers.h.al = saved_video_mode;
    int86(0x10, &registers, &registers);
}
