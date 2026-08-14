#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

void CB_FAR ship_3d_navigation_candidate_build(
        const volatile bloodprg_vm_object_header CB_FAR *target)
{
    const volatile cb_u16 CB_GAME_DATA *source;
    volatile cb_u16 *destination;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    cb_u16 object_offset;

    ship_3d_nav_source_list_build_full(
        target, (cb_u16 CB_NEAR *)ship_3d_nav_source_offsets);

    source = ship_3d_nav_source_offsets;
    destination = ship_3d_navigation_candidate_offsets;

    for (;;) {
        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
        if (object_offset == nav_choice_honk_record) {
            continue;
        }

        object = (const volatile bloodprg_vm_object_header CB_FAR *)
            (vm_record_base + object_offset);
        if (object->kind == SHIP_3D_SOURCE_BITSET_KIND &&
                (object->flags & 1u) != 0u) {
            *destination++ = object_offset;
        }
    }

    *destination = 0;
}
