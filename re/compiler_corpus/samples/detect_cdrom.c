/* Codegen probe for BLOODPRG 0x000B32. */

#include <dos.h>

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile unsigned char cdrom_present;

void NEAR detect_cdrom_probe(void)
{
    union REGS registers;

    registers.x.ax = 0x1500;
    registers.x.bx = 0;
    int86(0x2F, &registers, &registers);
    cdrom_present = (unsigned char)(registers.x.bx != 0);
}
