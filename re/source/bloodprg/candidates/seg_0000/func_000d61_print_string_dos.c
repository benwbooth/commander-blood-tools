#include <dos.h>

#include "../include/bloodprg_platform.h"

void CB_FAR print_string_dos(const volatile char *text)
{
    cb_u16 index;

    index = 0;
    while (text[index] != '\0') {
        bdos(2, (cb_u8)text[index], 0);
        ++index;
    }
}
