#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

#include "../include/bloodprg_hardware.h"

void CB_FAR vga_dac_clear(void)
{
    cb_u16 remaining;

    outportb(0x3C8, 0);
    remaining = 768u;
    do {
        outportb(0x3C9, 0);
    } while (--remaining != 0);
}
