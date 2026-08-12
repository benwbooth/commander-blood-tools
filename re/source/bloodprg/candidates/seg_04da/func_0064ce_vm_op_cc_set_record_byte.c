#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_cc_set_record_byte(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_i8 slot;
    volatile char CB_NEAR *destination;
    cb_u8 character;

    slot = (cb_i8)(cb_u8)(*script_bytes++ - 1u);
    destination = (volatile char CB_NEAR *)vm_record_string_slots
        + (cb_i16)slot * 16;

    do {
        character = *script_bytes++;
        *destination++ = (char)character;
    } while (character != 0);

    return script_bytes + 1;
}
