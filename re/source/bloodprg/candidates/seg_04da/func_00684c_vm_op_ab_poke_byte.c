#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_ab_poke_byte(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 value;
    volatile cb_u8 CB_NEAR *target;

    value = *script_bytes++;
    target = *(volatile cb_u8 CB_NEAR * const CB_NEAR *)script_bytes;
    *target = value;
    script_bytes += sizeof(target);
    return script_bytes;
}
