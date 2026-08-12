/* Codegen probe for BLOODPRG 0x00267D. */

#include <bios.h>

#if defined(__WATCOMC__)
#define keyboard_ready() _bios_keybrd(_KEYBRD_READY)
#define keyboard_read() _bios_keybrd(_KEYBRD_READ)
#else
#define keyboard_ready() bioskey(1)
#define keyboard_read() bioskey(0)
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

unsigned int FAR kbd_read_int16_probe(void)
{
    if (keyboard_ready() == 0) {
        return 0;
    }
    return keyboard_read();
}
