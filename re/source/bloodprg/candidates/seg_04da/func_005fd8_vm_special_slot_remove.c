#include "../include/bloodprg_vm.h"

int CB_NEAR vm_special_slot_remove(cb_u16 owner)
{
    volatile cb_u16 *slot;

    slot = vm_special_slots;
    do {
        if (*slot == owner) {
            *slot = 0;
            return 1;
        }
        ++slot;
    } while (slot != vm_special_slots + BLOODPRG_VM_SPECIAL_SLOT_COUNT);

    return 0;
}
