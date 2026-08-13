#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_FAR rtc_date_read(void)
{
    union REGS registers;
    cb_i16 year;

    registers.h.ah = 4u;
    int86(0x1a, &registers, &registers);

    rtc_day = (cb_i16)(cb_i8)bcd_to_binary(registers.h.dl);
    rtc_month = (cb_i16)(cb_i8)bcd_to_binary(registers.h.dh);
    year = (cb_i16)(cb_i8)bcd_to_binary(registers.h.cl);
    if (registers.h.ch == 0x13u) {
        year = (cb_i16)(year + 1900);
    } else {
        year = (cb_i16)(year + 2000);
    }
    rtc_year = year;
}
