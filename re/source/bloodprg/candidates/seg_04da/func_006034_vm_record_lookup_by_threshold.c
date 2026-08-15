#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_record_lookup_by_threshold(cb_u16 threshold)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;

    entry = vm_record_directory_gs;
    while (threshold > entry->object_offset) {
        ++entry;
    }
    --entry;

    return entry->object_offset;
}
