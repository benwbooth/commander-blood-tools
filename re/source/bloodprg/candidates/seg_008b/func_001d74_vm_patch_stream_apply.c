#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_IMAGE_SEGMENT_TYPE cb_u16
#define VM_IMAGE_SEGMENT(pointer) FP_SEG(pointer)
#define VM_IMAGE_AT(segment, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP((segment), (offset)))
#else
#define VM_IMAGE_SEGMENT_TYPE volatile cb_u8 CB_FAR *
#define VM_IMAGE_SEGMENT(pointer) (pointer)
#define VM_IMAGE_AT(segment, offset) ((segment) + (offset))
#endif

cb_u16 CB_NEAR vm_patch_stream_apply(cb_u16 byte_count)
{
    const volatile bloodprg_vm_patch_record CB_FAR *record;
    VM_IMAGE_SEGMENT_TYPE target_segment;
    cb_u16 target_offset;

    record = (const volatile bloodprg_vm_patch_record CB_FAR *)
            graphics_work_surface;
    target_segment = VM_IMAGE_SEGMENT(vm_script_image);

    do {
        target_offset = record->target_offset;
        *VM_IMAGE_AT(target_segment, target_offset) = record->value;
        ++record;
        byte_count = (cb_u16)(byte_count - 3u);
    } while (byte_count != 0);

    return target_offset;
}
