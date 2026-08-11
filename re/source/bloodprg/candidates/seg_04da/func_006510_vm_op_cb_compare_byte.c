#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_cb_compare_byte(const cb_u8 **script_bytes)
{
    cb_u8 tag;
    cb_u16 pair;
    cb_i8 high;
    cb_i8 low;
    cb_i8 compare_high;
    cb_i8 compare_low;
    int pass;

    tag = **script_bytes;
    ++*script_bytes;
    pair = *(const cb_u16 *)*script_bytes;
    *script_bytes += 4;

    high = (cb_i8)(pair >> 8);
    low = (cb_i8)(pair & 0xffu);
    compare_high = vm_compare_pair_high;
    compare_low = vm_compare_pair_low;

    if (tag == 0xf1u) {
        if (high > compare_high) {
            pass = 1;
        } else if (high < compare_high) {
            pass = 0;
        } else {
            pass = low > compare_low;
        }
    } else if (tag == 0xf2u) {
        if (high < compare_high) {
            pass = 1;
        } else if (high > compare_high) {
            pass = 0;
        } else {
            pass = low < compare_low;
        }
    } else {
        pass = high == compare_high && low == compare_low;
    }

    if (!pass) {
        vm_branch_fail();
    }
}
