#include "../include/bloodprg_vm.h"

void CB_NEAR scan_zero_word(bloodprg_vm_image_ptr script_bytes)
{
    const volatile cb_i16 CB_FAR *script_words;
    cb_u16 count;

    script_words = (const volatile cb_i16 CB_FAR *)script_bytes;
    count = 0;
    while (count != 0xffffu && *script_words > 0) {
        ++script_words;
        ++count;
    }

    vm_operand_word_count_gs = count;
}
