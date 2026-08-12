#include "../include/bloodprg_vm.h"

void CB_NEAR scan_zero_word(const cb_i16 CB_NEAR *script_words)
{
    cb_u16 count;

    count = 0;
    while (count != 0xffffu && *script_words > 0) {
        ++script_words;
        ++count;
    }

    vm_operand_word_count = count;
}
