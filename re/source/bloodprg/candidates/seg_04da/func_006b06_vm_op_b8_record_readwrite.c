#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_b8_record_readwrite(const cb_u16 **script_words)
{
    cb_u16 offset;
    cb_u16 first;
    cb_u16 second;
    cb_u16 threshold;
    volatile cb_u16 CB_FAR *field;
    volatile cb_u16 CB_FAR *secondary_link;

    offset = **script_words;
    ++*script_words;
    first = **script_words;
    ++*script_words;
    second = **script_words;
    ++*script_words;

    field = (volatile cb_u16 CB_FAR *)(vm_record_base + offset);
    if ((vm_query_mode & 1u) != 0) {
        if (field[0] != first || field[1] != second) {
            vm_branch_fail();
        }
    } else {
        field[0] = first;
        field[1] = second;
        threshold = vm_record_lookup_by_threshold(offset);
        secondary_link = (volatile cb_u16 CB_FAR *)(vm_secondary_record + 0x16);
        if (threshold == *secondary_link) {
            *secondary_link = 0;
        }
    }
}
