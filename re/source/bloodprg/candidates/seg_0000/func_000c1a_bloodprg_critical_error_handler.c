#include <dos.h>

#include "../include/bloodprg_hardware.h"

void CB_INTERRUPT CB_FAR bloodprg_critical_error_handler(cb_u16 error_code)
{
    dos_critical_error_code_plus_one = error_code;
    ++dos_critical_error_code_plus_one;
    _enable();
}
