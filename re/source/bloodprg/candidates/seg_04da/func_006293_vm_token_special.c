#include "../include/bloodprg_vm.h"

void CB_NEAR vm_token_special(const cb_u8 **script_bytes, cb_u16 terminator)
{
    while (*(const cb_u16 *)*script_bytes != terminator) {
        ++*script_bytes;
    }

    *script_bytes += 2;
    if (**script_bytes == (cb_u8)terminator) {
        ++*script_bytes;
    }
}
