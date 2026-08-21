#include "../include/bloodprg_vm.h"

const cb_u16 CB_NEAR *CB_NEAR vm_op_ca_compare_var(
    const cb_u16 CB_NEAR *script_words)
{
    cb_u8 operator;
    cb_i16 value;

    operator = (cb_u8)*script_words++;
    value = (cb_i16)*script_words++;

    if (operator == 0xf1u) {
        if (value > rtc_hour) {
            return script_words;
        }
    } else if (operator == 0xf2u) {
        if (value < rtc_hour) {
            return script_words;
        }
    } else if (value == rtc_hour) {
        return script_words;
    }

    return (const cb_u16 CB_NEAR *)vm_branch_fail();
}
