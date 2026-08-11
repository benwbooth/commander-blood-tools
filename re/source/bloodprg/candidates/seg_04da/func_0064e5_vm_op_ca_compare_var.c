#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_ca_compare_var(const cb_u16 **script_words)
{
    cb_u8 tag;
    cb_i16 value;
    cb_i16 compare;
    int pass;

    tag = (cb_u8)**script_words;
    ++*script_words;
    value = (cb_i16)**script_words;
    ++*script_words;

    compare = vm_compare_word;
    if (tag == 0xf1u) {
        pass = value > compare;
    } else if (tag == 0xf2u) {
        pass = value < compare;
    } else {
        pass = value == compare;
    }

    if (!pass) {
        vm_branch_fail();
    }
}
