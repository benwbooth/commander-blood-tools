#include <dos.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR install_ctrl_break_handler(void)
{
    _dos_setvect(0x23u, bloodprg_ctrl_break_handler);
    _dos_setvect(0x24u, bloodprg_critical_error_handler);
}
