/* Codegen probe for BLOODPRG 0x00079C. */

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
extern volatile unsigned char timer_divider;
extern volatile unsigned int timer_reload_ticks;
extern volatile unsigned int timer_subtick_limit;
extern void INTERRUPT FAR timer_isr(void);

void FAR install_timer_isr_hook_probe(void)
{
    timer_previous_handler = _dos_getvect(0x08u);
    _dos_setvect(0x08u, timer_isr);

    _disable();
    outp(0x0043u, 0x36u);
    outp(0x0040u, 0x46u);
    outp(0x0040u, 0x17u);
    timer_hook_active = 1u;
    timer_divider = 0x0bu;
    timer_subtick_limit = 0x0019u;
    timer_reload_ticks = 0x0003u;
    _enable();
}
