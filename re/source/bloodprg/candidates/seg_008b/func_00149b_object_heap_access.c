#include "../include/bloodprg_vm.h"

void CB_NEAR object_heap_access(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    volatile bloodprg_vm_object_record CB_FAR *object;

    entry = vm_record_directory;
    do {
        object = (volatile bloodprg_vm_object_record CB_FAR *)
            (vm_record_base + entry->object_offset);
        if ((object->kind & BLOODPRG_VM_OBJECT_ACCESS_MASK) != 0 &&
                (object->flags & BLOODPRG_VM_OBJECT_IN_PLAY_FLAG) != 0) {
            ++object->access_count;
        }
        ++entry;
    } while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND);
}
