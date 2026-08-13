#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_SCRIPT_SEGMENT_TYPE cb_u16
#define VM_SCRIPT_SEGMENT(pointer) FP_SEG(pointer)
#define VM_SCRIPT_AT(segment, offset) \
    ((const volatile cb_u8 CB_FAR *)MK_FP((segment), (offset)))
#else
#define VM_SCRIPT_SEGMENT_TYPE const volatile cb_u8 CB_FAR *
#define VM_SCRIPT_SEGMENT(pointer) (pointer)
#define VM_SCRIPT_AT(segment, offset) ((segment) + (offset))
#endif

cb_u16 CB_NEAR vm_patch_stream_build(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    volatile bloodprg_vm_patch_record CB_FAR *record;
    VM_SCRIPT_SEGMENT_TYPE script_segment;
    cb_u16 byte_count;
    cb_u16 object_offset;

    record = (volatile bloodprg_vm_patch_record CB_FAR *)
            graphics_work_surface;
    entry = vm_record_directory_gs;
    script_segment = VM_SCRIPT_SEGMENT(vm_script_image);
    byte_count = 0;

    while (entry->object_offset != 0xffffu) {
        if (entry->entry_kind == 2u) {
            object_offset = entry->object_offset;
            record->target_offset = object_offset;
            record->value = *VM_SCRIPT_AT(script_segment, object_offset);
            ++record;
            byte_count = (cb_u16)(byte_count
                    + (cb_u16)sizeof(bloodprg_vm_patch_record));
        }
        ++entry;
    }

    return byte_count;
}
