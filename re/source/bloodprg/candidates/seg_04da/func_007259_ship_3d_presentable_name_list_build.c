#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define SHIP_3D_PRESENTABLE_OBJECT_AT(target, offset) \
    ((const volatile bloodprg_vm_object_header CB_FAR *) \
        MK_FP(FP_SEG(target), (offset)))
#define SHIP_3D_PRESENTABLE_OBJECT_OFFSET(target) ((cb_u16)FP_OFF(target))
#else
#define SHIP_3D_PRESENTABLE_OBJECT_AT(target, offset) \
    ((const volatile bloodprg_vm_object_header *)(vm_record_base + (offset)))
#define SHIP_3D_PRESENTABLE_OBJECT_OFFSET(target) \
    ((cb_u16)((const volatile cb_u8 *)(target) - vm_record_base))
#endif

volatile cb_u16 CB_NEAR *CB_FAR ship_3d_presentable_name_list_build(
        const volatile bloodprg_vm_object_header CB_FAR *target)
{
    const volatile cb_u16 CB_GAME_DATA *source;
    volatile cb_u16 *destination;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    cb_u16 object_offset;

    ship_3d_nav_source_list_build_full(
        target, ship_3d_nav_source_offsets);

    source = ship_3d_nav_source_offsets;
    destination = ship_3d_presentable_name_offsets;
    object_offset = SHIP_3D_PRESENTABLE_OBJECT_OFFSET(target);

    for (;;) {
        object = SHIP_3D_PRESENTABLE_OBJECT_AT(target, object_offset);
        if ((object->kind & SHIP_3D_PRESENTABLE_KIND_MASK) != 0u &&
                (object->flags & BLOODPRG_VM_OBJECT_IN_PLAY_FLAG) != 0u &&
                object_offset != vm_arche_record_offset) {
            *destination++ =
                (cb_u16)(object_offset + SHIP_3D_OBJECT_NAME_OFFSET);
        }

        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
    }

    *destination = 0xffffu;
    return destination;
}
