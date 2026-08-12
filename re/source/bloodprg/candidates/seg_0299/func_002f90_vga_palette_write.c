#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

#include "../include/bloodprg_hardware.h"

void CB_FAR vga_palette_write(const volatile cb_u8 *palette)
{
    cb_u16 index;

    outportb(0x3C8, 0);
    for (index = 0; index < 768u; ++index) {
        outportb(0x3C9, palette[index]);
    }
}
