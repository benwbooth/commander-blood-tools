#include "../include/bloodprg_nav.h"

cb_u16 CB_FAR nav_kind2_target_list_build(void)
{
    const volatile cb_u16 *source;
    volatile cb_u16 *destination;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    cb_u16 object_offset;
    cb_u16 count;

    count = 0;
    active_object_list_build();
    source = vm_active_object_offsets;
    destination = nav_kind2_target_offsets;

    for (;;) {
        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
        if (object_offset == nav_choice_honk_record ||
                object_offset == nav_choice_radio_record) {
            continue;
        }

        object = (const volatile bloodprg_vm_object_header CB_FAR *)
            (vm_record_base + object_offset);
        if (object->kind == 2u) {
            *destination++ = object_offset;
            ++count;
        }
    }

    *destination = 0xffffu;
    return count;
}
