#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_c6_record_match(const cb_u8 **script_bytes)
{
    int inverted;
    cb_u16 record_offset;
    cb_u16 operand;
    const cb_u16 *script_words;
    volatile cb_u16 CB_FAR *record;
    int matches;

    inverted = 0;
    if (**script_bytes == 0xa1u) {
        inverted = 1;
        ++*script_bytes;
    }

    script_words = (const cb_u16 *)*script_bytes;
    record_offset = *script_words++;
    operand = *script_words++;
    *script_bytes = (const cb_u8 *)script_words;

    record = (volatile cb_u16 CB_FAR *)(vm_record_base + record_offset);
    if ((vm_query_mode & 1u) != 0) {
        matches = record[1] == operand && record[0] == 0x00c6u;
        if (matches == inverted) {
            vm_branch_fail();
        }
    } else {
        record[0] = 0x00c6u;
        record[1] = operand;
        record[2] = 0;
    }
}
