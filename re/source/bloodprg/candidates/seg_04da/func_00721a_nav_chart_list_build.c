#include "../include/bloodprg_vm.h"

cb_u16 CB_FAR nav_chart_list_build(void)
{
    const volatile cb_u16 *source;
    volatile cb_u16 *destination;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    cb_u16 object_offset;
    cb_u16 count;

    count = 0;
    active_object_list_build();
    source = vm_active_object_offsets;
    destination = vm_nav_chart_object_offsets;

    for (;;) {
        object_offset = *source++;
        if ((cb_i16)object_offset < 0) {
            break;
        }

        object = (const volatile bloodprg_vm_object_header CB_FAR *)
            (vm_record_base + object_offset);
        if ((object->kind & BLOODPRG_VM_OBJECT_ACCESS_MASK) != 0) {
            *destination++ = object_offset;
            ++count;
        }
    }

    *destination = object_offset;
    return count;
}
