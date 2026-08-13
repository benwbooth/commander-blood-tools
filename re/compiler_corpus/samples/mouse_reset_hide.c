/* Codegen probe for BLOODPRG 0x000CEF. */

#include <dos.h>

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

void FAR mouse_reset_hide_probe(void)
{
    union REGS registers;

    registers.x.ax = 0u;
    int86(0x33, &registers, &registers);

    registers.x.ax = 2u;
    int86(0x33, &registers, &registers);

    registers.x.ax = 0x000fu;
    registers.x.cx = 0x000cu;
    registers.x.dx = 0x000cu;
    int86(0x33, &registers, &registers);
}
