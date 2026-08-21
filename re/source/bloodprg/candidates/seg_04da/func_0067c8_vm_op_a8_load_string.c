#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a8_load_string(
    bloodprg_vm_image_ptr script_bytes)
{
    volatile cb_u8 CB_GAME_DATA *dst;
    cb_u8 ch;

    dst = vm_load_string_buffer_gs;
    do {
        ch = *script_bytes++;
        *dst++ = ch;
    } while (ch != '\0');

    ++script_bytes;

    if (vm_load_string_buffer_gs[0] == 'f'
            && vm_load_string_buffer_gs[1] == 'i'
            && vm_load_string_buffer_gs[2] == 'n'
            && vm_load_string_buffer_gs[3] == '.') {
        vm_finale_requested_gs = 1;
    }

    if ((vm_presentation_request_flags_gs & 2u) == 0
            && ((vm_ship_active_flags_gs & 1u) != 0
                || (vm_scene_gate_gs & 1u) != 0)) {
        vm_active_line_gs = 7;
        vm_presentation_request_flags_gs |= 2u;
        vm_c2_presentation_gate_gs = 0;
        vm_loaded_scene_image_path_offset_gs = 0xffffu;
        vm_dialog_gate_0b3b_gs = 0;
    }

    return script_bytes;
}
