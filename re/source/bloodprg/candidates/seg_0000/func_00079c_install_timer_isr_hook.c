#include <dos.h>
#include <conio.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR install_timer_isr_hook(void)
{
    timer_previous_handler = _dos_getvect(0x08u);
    _dos_setvect(0x08u, bloodprg_timer_isr);

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
