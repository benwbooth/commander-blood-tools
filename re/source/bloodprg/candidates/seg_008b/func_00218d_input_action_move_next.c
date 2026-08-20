#include <string.h>

#include "../include/bloodprg_input.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_vm.h"

#define INPUT_SELECTION_ACTIVE_MASK 0x03u
#define INPUT_SELECTION_DIRECTORY_POINTER 0x01u
#define INPUT_SELECTION_NONE 0xffffu
#define INPUT_SELECTION_VISIBLE_ROWS 15
#define INPUT_SAVE_LAST_EDITABLE_SLOT 8u

void CB_NEAR input_action_move_next(cb_u8 raw_low_byte)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *directory;
    cb_u16 selected;

    (void)raw_low_byte;
    if ((input_selection_mode_flags & INPUT_SELECTION_ACTIVE_MASK) != 0u) {
        if (input_directory_selection_offset != INPUT_SELECTION_NONE) {
            return;
        }

        selected = (cb_u16)(vm_profile_word_67a2 + 1u);
        if ((input_selection_mode_flags
                & INPUT_SELECTION_DIRECTORY_POINTER) != 0u) {
            directory = vm_record_directory_gs;
        } else {
            directory = vm_default_record_directory;
        }
        if (directory[(cb_u8)selected].entry_kind == 0u) {
            return;
        }

        vm_profile_word_67a2 = selected;
        if ((cb_i16)(selected - vm_profile_word_67a0)
                >= INPUT_SELECTION_VISIBLE_ROWS) {
            ++vm_profile_word_67a0;
        }
        return;
    }

    if ((save_request_active & 1u) != 0u
            && save_slot_selected_index != INPUT_SAVE_LAST_EDITABLE_SLOT) {
        ++save_slot_selected_index;
        save_slot_active_name += sizeof(bloodprg_save_slot);
        memcpy(
                (void CB_NEAR *)save_slot_edit_buffer,
                (const void CB_NEAR *)save_slot_active_name,
                BLOODPRG_SAVE_SLOT_NAME_BYTES);
    }
}
