/* Codegen probe for BLOODPRG 0x0007EA. */

#include <conio.h>
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

typedef void (INTERRUPT FAR *interrupt_handler)(void);

extern interrupt_handler timer_previous_handler;
extern volatile unsigned char timer_hook_active;

void FAR restore_timer_isr_hook_probe(void)
{
    _disable();
    outp(0x0043u, 0x36u);
    outp(0x0040u, 0xffu);
    outp(0x0040u, 0xffu);
    timer_hook_active = 0u;
    _enable();

    _dos_setvect(0x08u, timer_previous_handler);
}
