#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_token_special(cb_u16 terminator,
        bloodprg_vm_image_ptr script_bytes)
{
    while (*(const volatile cb_u16 CB_FAR *)script_bytes != terminator) {
        ++script_bytes;
    }

    script_bytes += 2;
    if (*script_bytes == (cb_u8)terminator) {
        ++script_bytes;
    }

    return script_bytes;
}
