#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a7_set_if_presentation(const cb_u16 **script_words)
{
    cb_u16 value;

    value = **script_words;
    ++*script_words;

    if ((vm_presentation_active & 1u) != 0) {
        vm_presentation_reg_6770 = value;
    }
}
