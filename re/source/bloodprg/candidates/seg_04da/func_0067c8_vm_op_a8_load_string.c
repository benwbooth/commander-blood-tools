#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a8_load_string(
    const cb_u8 CB_NEAR *script_bytes)
{
    volatile cb_u8 *dst;
    cb_u8 ch;

    dst = vm_load_string_buffer;
    do {
        ch = *script_bytes++;
        *dst++ = ch;
    } while (ch != '\0');

    ++script_bytes;

    if (vm_load_string_buffer[0] == 'f'
            && vm_load_string_buffer[1] == 'i'
            && vm_load_string_buffer[2] == 'n'
            && vm_load_string_buffer[3] == '.') {
        vm_finale_requested = 1;
    }

    if ((vm_presentation_request_flags & 2u) == 0
            && ((vm_ship_active_flags & 1u) != 0 || (vm_scene_gate & 1u) != 0)) {
        vm_active_line = 7;
        vm_presentation_request_flags |= 2u;
        vm_c2_presentation_gate = 0;
        vm_loaded_scene_image_path = (volatile char CB_NEAR *)0xffffu;
        vm_dialog_gate_0b3b = 0;
    }

    return script_bytes;
}
