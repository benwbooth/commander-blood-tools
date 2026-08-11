#include "../include/bloodprg_vm.h"

int CB_NEAR vm_field_offset(cb_u16 selector, cb_u16 kind_mask)
{
    cb_u16 bit_index;

    bit_index = 0;
    while ((kind_mask & (1u << bit_index)) == 0) {
        ++bit_index;
    }

    return (int)vm_field_offset_table[(selector << 4) + bit_index];
}
