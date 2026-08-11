#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_ab_poke_byte(const cb_u8 **script_bytes)
{
    cb_u8 value;
    volatile cb_u8 *target;
    volatile cb_u8 * const *target_slot;

    value = **script_bytes;
    ++*script_bytes;

    target_slot = (volatile cb_u8 * const *)*script_bytes;
    target = *target_slot;
    *target = value;
    *script_bytes += sizeof(target);
}
