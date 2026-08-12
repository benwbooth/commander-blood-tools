#include <bios.h>

#if defined(__WATCOMC__)
#define keyboard_ready() _bios_keybrd(_KEYBRD_READY)
#define keyboard_read() _bios_keybrd(_KEYBRD_READ)
#else
#define keyboard_ready() bioskey(1)
#define keyboard_read() bioskey(0)
#endif

#include "../include/bloodprg_platform.h"

cb_u16 CB_FAR kbd_read_int16(void)
{
    if (keyboard_ready() == 0) {
        return 0;
    }
    return keyboard_read();
}
