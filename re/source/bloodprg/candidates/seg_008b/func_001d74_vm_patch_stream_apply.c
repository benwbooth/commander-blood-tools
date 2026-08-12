#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

void CB_NEAR vm_patch_stream_apply(cb_u16 byte_count)
{
    const volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *target;
    cb_u16 target_offset;

    source = graphics_work_surface;
    target = vm_script_image;

    do {
        target_offset = *(const volatile cb_u16 CB_FAR *)source;
        source += 2;
        target[target_offset] = *source;
        ++source;
        byte_count = (cb_u16)(byte_count - 3u);
    } while (byte_count != 0);
}
