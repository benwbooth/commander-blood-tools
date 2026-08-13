#include <dos.h>
#include <conio.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR restore_timer_isr_hook(void)
{
    _disable();
    outp(0x0043u, 0x36u);
    outp(0x0040u, 0xffu);
    outp(0x0040u, 0xffu);
    timer_hook_active = 0u;
    _enable();

    _dos_setvect(0x08u, timer_previous_handler);
}
