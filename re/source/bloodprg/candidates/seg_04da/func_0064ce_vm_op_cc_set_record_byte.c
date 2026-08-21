#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_cc_set_record_byte(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_i8 slot;
    volatile char CB_GAME_DATA *destination;
    cb_u8 character;

    slot = (cb_i8)(cb_u8)(*script_bytes++ - 1u);
    destination = vm_record_string_slots[(cb_i16)slot];

    do {
        character = *script_bytes++;
        *destination++ = (char)character;
    } while (character != 0);

    return script_bytes + 1;
}
