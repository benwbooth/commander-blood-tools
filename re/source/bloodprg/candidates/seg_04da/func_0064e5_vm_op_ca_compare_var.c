#include "../include/bloodprg_vm.h"

const cb_u16 CB_NEAR *CB_NEAR vm_op_ca_compare_var(
    const cb_u16 CB_NEAR *script_words)
{
    cb_u8 tag;
    cb_i16 value;

    tag = (cb_u8)*script_words++;
    value = (cb_i16)*script_words++;

    if (tag == 0xf1u) {
        if (value > vm_compare_word) {
            return script_words;
        }
    } else if (tag == 0xf2u) {
        if (value < vm_compare_word) {
            return script_words;
        }
    } else if (value == vm_compare_word) {
        return script_words;
    }

    return (const cb_u16 CB_NEAR *)vm_branch_fail();
}
