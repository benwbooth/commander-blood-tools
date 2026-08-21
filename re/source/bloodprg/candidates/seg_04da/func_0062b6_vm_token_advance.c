#include <dos.h>

#include "../include/bloodprg_vm.h"

#define VM_DESC_SET_MODE ((cb_i8)-1)
#define VM_DESC_CLEAR_MODE ((cb_i8)-2)
#define VM_DESC_OPTIONAL_PREFIX ((cb_i8)-3)
#define VM_DESC_SCAN_OR_PREFIX ((cb_i8)-5)

bloodprg_vm_image_ptr CB_NEAR vm_token_advance(
        bloodprg_vm_image_ptr script_bytes)
{
    const bloodprg_vm_opcode_descriptor CB_GAME_DATA *descriptor;
    const cb_u8 CB_GAME_DATA *mode_lengths;
    cb_u8 opcode;
    cb_u8 length;
    cb_i8 control;

    opcode = *script_bytes++;
    descriptor = vm_opcode_descriptors
            + (cb_i8)(opcode - BLOODPRG_VM_OPCODE_MIN);
    control = descriptor->mode_one_length_or_control;

    if (control >= 0) {
        mode_lengths = (const cb_u8 CB_GAME_DATA *)descriptor;
        length = mode_lengths[vm_query_mode_gs];
    } else if (control == VM_DESC_SET_MODE) {
        vm_query_mode_gs = 1;
        length = descriptor->mode_zero_length;
    } else if (control == VM_DESC_CLEAR_MODE) {
        vm_query_mode_gs = 0;
        length = descriptor->mode_zero_length;
    } else if (control == VM_DESC_OPTIONAL_PREFIX) {
        if (*script_bytes == BLOODPRG_VM_OPTION_PREFIX) {
            ++script_bytes;
        }
        length = descriptor->mode_zero_length;
    } else if ((vm_block_scan_flags_gs & 1u) != 0) {
        length = 0;
    } else {
        if (control == VM_DESC_SCAN_OR_PREFIX
                && *script_bytes == BLOODPRG_VM_OPTION_PREFIX) {
            ++script_bytes;
        }
        length = descriptor->mode_zero_length;
    }

    if (length != 0) {
        return script_bytes + (cb_i8)(cb_u8)(length - 1u);
    }

    if (opcode == BLOODPRG_VM_TEXT_OPCODE) {
        script_bytes += 5;
        while (*(const volatile cb_u16 CB_FAR *)script_bytes != 0) {
            script_bytes += 2;
        }
        return script_bytes + 2;
    }

    return vm_token_special(0, script_bytes);
}
