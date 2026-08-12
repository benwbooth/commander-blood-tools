#include "../include/bloodprg_vm.h"

typedef union bloodprg_vm_compare_pair {
    cb_u16 word;
    struct {
        cb_i8 low;
        cb_i8 high;
    } bytes;
} bloodprg_vm_compare_pair;

const cb_u8 CB_NEAR *CB_NEAR vm_op_cb_compare_byte(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 tag;
    bloodprg_vm_compare_pair pair;

    tag = *script_bytes++;
    pair.word = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += 4;

    if (tag == 0xf1u) {
        if (pair.bytes.high < vm_compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.high > vm_compare_pair_high) {
            return script_bytes;
        }
        if (pair.bytes.low <= vm_compare_pair_low) {
            goto failed;
        }
        return script_bytes;
    } else if (tag == 0xf2u) {
        if (pair.bytes.high > vm_compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.high < vm_compare_pair_high) {
            return script_bytes;
        }
        if (pair.bytes.low >= vm_compare_pair_low) {
            goto failed;
        }
        return script_bytes;
    } else {
        if (pair.bytes.high != vm_compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.low == vm_compare_pair_low) {
            return script_bytes;
        }
    }

failed:
    return (const cb_u8 CB_NEAR *)vm_branch_fail();
}
