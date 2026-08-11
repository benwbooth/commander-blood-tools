#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_cc_set_record_byte(const cb_i8 **script_bytes)
{
    int slot;
    volatile char *dst;
    char ch;

    slot = (int)**script_bytes - 1;
    ++*script_bytes;
    dst = &vm_record_string_slots[slot][0];

    do {
        ch = (char)**script_bytes;
        ++*script_bytes;
        *dst = ch;
        ++dst;
    } while (ch != '\0');

    ++*script_bytes;
}
