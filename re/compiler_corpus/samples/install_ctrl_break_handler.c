/* Codegen probe for BLOODPRG 0x000BFF. */

#include <dos.h>

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define INTERRUPT __interrupt
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define INTERRUPT interrupt
#else
#define INTERRUPT
#endif

extern void INTERRUPT FAR ctrl_break_handler(void);
extern void INTERRUPT FAR critical_error_handler(void);

void FAR install_ctrl_break_handler_probe(void)
{
    _dos_setvect(0x23u, ctrl_break_handler);
    _dos_setvect(0x24u, critical_error_handler);
}
