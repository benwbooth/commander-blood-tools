/* Codegen probe for BLOODPRG 0x002F90. */

#if defined(__WATCOMC__)
#include <conio.h>
#define outportb outp
#else
#include <dos.h>
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#pragma aux vga_palette_write_probe parm [si]
#endif

void FAR vga_palette_write_probe(const volatile unsigned char *palette)
{
    unsigned int index;

    outportb(0x3C8, 0);
    for (index = 0; index < 768u; ++index) {
        outportb(0x3C9, palette[index]);
    }
}
