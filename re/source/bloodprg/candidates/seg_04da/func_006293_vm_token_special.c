#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_token_special(cb_u16 terminator,
        const cb_u8 CB_NEAR *script_bytes)
{
    while (*(const cb_u16 CB_NEAR *)script_bytes != terminator) {
        ++script_bytes;
    }

    script_bytes += 2;
    if (*script_bytes == (cb_u8)terminator) {
        ++script_bytes;
    }

    return script_bytes;
}
