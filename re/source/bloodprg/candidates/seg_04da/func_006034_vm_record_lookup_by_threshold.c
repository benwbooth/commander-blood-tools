#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_record_lookup_by_threshold(cb_u16 threshold)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    const volatile bloodprg_vm_directory_entry CB_FAR *previous;

    entry = vm_record_directory;
    previous = entry - 1;
    while (threshold > entry->object_offset) {
        previous = entry;
        ++entry;
    }

    return previous->object_offset;
}
