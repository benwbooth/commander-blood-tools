/* Codegen probe for BLOODPRG 0x002DD3. */

#if defined(__WATCOMC__)
#include <conio.h>
#define inportb inp
#define outportb outp
#else
#include <dos.h>
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

extern volatile unsigned int cmos_seconds_pair;

void FAR cmos_rtc_read_probe(void)
{
    unsigned char seconds;

    outportb(0x70, 0);
    seconds = inportb(0x71);
    cmos_seconds_pair = (unsigned int)(
        seconds | ((unsigned int)seconds << 8));
}
