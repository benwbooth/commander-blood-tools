#include <string.h>

#include "../include/bloodprg_input.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_vm.h"

#define INPUT_SELECTION_ACTIVE_MASK 0x03u
#define INPUT_SELECTION_NONE 0xffffu

void CB_NEAR input_action_move_previous(cb_u8 raw_low_byte)
{
    cb_u16 selected;

    (void)raw_low_byte;
    if ((input_selection_mode_flags & INPUT_SELECTION_ACTIVE_MASK) != 0u) {
        if (input_directory_selection_offset != INPUT_SELECTION_NONE) {
            return;
        }

        selected = vm_profile_word_67a2;
        if (selected == 0u) {
            return;
        }

        --selected;
        if ((cb_i16)(selected - vm_profile_word_67a0) < 0) {
            --vm_profile_word_67a0;
        }
        vm_profile_word_67a2 = selected;
        return;
    }

    if ((save_request_active & 1u) != 0u
            && save_slot_selected_index != 0u) {
        --save_slot_selected_index;
        save_slot_active_name -= sizeof(bloodprg_save_slot);
        memcpy(
                (void CB_NEAR *)save_slot_edit_buffer,
                (const void CB_NEAR *)save_slot_active_name,
                BLOODPRG_SAVE_SLOT_NAME_BYTES);
    }
}
