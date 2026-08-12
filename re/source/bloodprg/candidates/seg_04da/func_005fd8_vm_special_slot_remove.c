#include "../include/bloodprg_vm.h"

int CB_NEAR vm_special_slot_remove(cb_u16 owner)
{
    cb_u16 i;

    for (i = 0; i < 16u; ++i) {
        if (vm_special_slots[i] == owner) {
            vm_special_slots[i] = 0;
            return 1;
        }
    }

    return 0;
}
