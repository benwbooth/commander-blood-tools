/* Codegen probe for BLOODPRG 0x002FA6. */

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

void FAR vga_dac_clear_probe(void)
{
    unsigned int remaining;

    outportb(0x3C8, 0);
    remaining = 768u;
    do {
        outportb(0x3C9, 0);
    } while (--remaining != 0);
}
