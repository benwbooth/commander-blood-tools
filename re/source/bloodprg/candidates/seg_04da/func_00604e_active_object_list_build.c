#include "../include/bloodprg_vm.h"

void CB_NEAR active_object_list_build(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    volatile cb_u16 *out;

    out = vm_active_object_offsets;
    entry = vm_record_directory;
    while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
        object = (const volatile bloodprg_vm_object_header CB_FAR *)
            (vm_record_base + entry->object_offset);
        if ((object->flags & BLOODPRG_VM_OBJECT_IN_PLAY_FLAG) != 0) {
            *out = entry->object_offset;
            ++out;
        }
        ++entry;
    }

    *out = 0xffffu;
}
