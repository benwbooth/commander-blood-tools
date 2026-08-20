#include "../include/bloodprg_input.h"
#include "../include/bloodprg_vm.h"

#define INPUT_SELECTION_DIRECTORY_POINTER 0x01u

void CB_NEAR input_action_accept(cb_u8 raw_low_byte)
{
    cb_u16 record_offset;

    input_dispatch_state_b15 = raw_low_byte;
    if ((input_selection_mode_flags
            & INPUT_SELECTION_DIRECTORY_POINTER) == 0u) {
        return;
    }

    record_offset = (cb_u16)((cb_u8)vm_profile_word_67a2
            * (cb_u16)sizeof(bloodprg_vm_directory_entry));
    if (vm_record_directory_gs[(cb_u8)vm_profile_word_67a2].entry_kind
            == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
        input_directory_selection_offset = record_offset;
    }
}
