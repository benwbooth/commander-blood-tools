#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a2_cond_call(const cb_u16 **script_words)
{
    cb_u16 modulus;

    modulus = **script_words;
    ++*script_words;

    if (blood_prng_next(modulus) != 0) {
        vm_branch_fail();
    }
}
