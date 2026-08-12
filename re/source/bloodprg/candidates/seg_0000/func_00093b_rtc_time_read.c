#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_FAR rtc_time_read(void)
{
    union REGS registers;
    cb_u8 hour;

    registers.x.ax = 0x0200;
    int86(0x1A, &registers, &registers);
    hour = bcd_to_binary(registers.h.ch);
    rtc_hour = (cb_i16)(cb_i8)hour;
}
