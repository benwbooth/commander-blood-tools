#if defined(__WATCOMC__)
#include <conio.h>
#define inportb inp
#define outportb outp
#else
#include <dos.h>
#endif

#include "../include/bloodprg_hardware.h"

void CB_FAR cmos_rtc_read(void)
{
    cb_u8 seconds;

    outportb(0x70, 0);
    seconds = inportb(0x71);
    cmos_seconds_pair = (cb_u16)(seconds | ((cb_u16)seconds << 8));
}
