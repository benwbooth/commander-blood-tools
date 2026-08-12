#include "../include/bloodprg_vm.h"

const cb_u16 CB_NEAR *CB_NEAR vm_op_a2_cond_call(
    const cb_u16 CB_NEAR *script_words)
{
    cb_u16 modulus;

    modulus = *script_words++;

    if (blood_prng_next(modulus) != 0) {
        return (const cb_u16 CB_NEAR *)vm_branch_fail();
    }

    return script_words;
}
