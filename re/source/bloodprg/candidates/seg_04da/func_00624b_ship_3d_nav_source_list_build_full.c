#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define SHIP_3D_OBJECT_AT(target, offset) \
    ((const volatile bloodprg_vm_object_header CB_FAR *) \
        MK_FP(FP_SEG(target), (offset)))
#define SHIP_3D_OBJECT_OFFSET(target) ((cb_u16)FP_OFF(target))
#else
#define SHIP_3D_OBJECT_AT(target, offset) \
    ((const volatile bloodprg_vm_object_header *)(vm_record_base + (offset)))
#define SHIP_3D_OBJECT_OFFSET(target) \
    ((cb_u16)((const volatile cb_u8 *)(target) - vm_record_base))
#endif

volatile cb_u16 CB_GAME_DATA *CB_FAR ship_3d_nav_source_list_build_full(
        const volatile bloodprg_vm_object_header CB_FAR *target,
        volatile cb_u16 CB_GAME_DATA *output)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    const volatile cb_u16 CB_FAR *parent;
    cb_u16 field_offset;

    entry = vm_record_directory_gs;
    do {
        object = SHIP_3D_OBJECT_AT(target, entry->object_offset);
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_PARENT_LINK, object->kind);
        if (field_offset != 0) {
            parent = (const volatile cb_u16 CB_FAR *)
                ((const volatile cb_u8 CB_FAR *)object + field_offset);
            if (*parent == SHIP_3D_OBJECT_OFFSET(target)) {
                *output = entry->object_offset;
                ++output;
                output = ship_3d_nav_source_list_build_full(object, output);
            }
        }

        ++entry;
    } while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND);

    *output = 0xffffu;
    return output;
}
