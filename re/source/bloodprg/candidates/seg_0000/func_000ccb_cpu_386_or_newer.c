#include "../include/bloodprg_hardware.h"

cb_u16 CB_FAR cpu_386_or_newer(void)
{
    cb_u16 original_flags;
    cb_u16 observed_flags;
    cb_u16 supported;

    original_flags = cb_flags_read();
    supported = 0u;

    cb_flags_write(0u);
    observed_flags = cb_flags_read();
    if ((observed_flags & 0xf000u) != 0xf000u) {
        cb_flags_write(0x7000u);
        observed_flags = cb_flags_read();
        if ((observed_flags & 0x7000u) != 0u) {
            supported = 1u;
        }
    }

    cb_flags_write(original_flags);
    return supported;
}
