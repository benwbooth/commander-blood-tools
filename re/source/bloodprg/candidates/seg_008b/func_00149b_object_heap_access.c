#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define BLOODPRG_OBJECT_HEAP_SEGMENT_TYPE cb_u16
#define BLOODPRG_OBJECT_HEAP_SEGMENT(pointer) FP_SEG(pointer)
#define BLOODPRG_OBJECT_HEAP_AT(segment, offset) \
    ((volatile bloodprg_vm_object_record CB_FAR *) \
        MK_FP((segment), (offset)))
#else
#define BLOODPRG_OBJECT_HEAP_SEGMENT_TYPE volatile cb_u8 CB_FAR *
#define BLOODPRG_OBJECT_HEAP_SEGMENT(pointer) (pointer)
#define BLOODPRG_OBJECT_HEAP_AT(segment, offset) \
    ((volatile bloodprg_vm_object_record CB_FAR *) \
        ((segment) + (offset)))
#endif

void CB_SAVE_REGS CB_NEAR object_heap_access(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    volatile bloodprg_vm_object_record CB_FAR *object;
    BLOODPRG_OBJECT_HEAP_SEGMENT_TYPE object_segment;

    object_segment = BLOODPRG_OBJECT_HEAP_SEGMENT(vm_record_base);
    entry = vm_record_directory;
    do {
        object = BLOODPRG_OBJECT_HEAP_AT(object_segment, entry->object_offset);
        if ((object->kind & BLOODPRG_VM_OBJECT_ACCESS_MASK) != 0 &&
                (object->flags & BLOODPRG_VM_OBJECT_IN_PLAY_FLAG) != 0) {
            ++object->access_count;
        }
        ++entry;
    } while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND);
}
