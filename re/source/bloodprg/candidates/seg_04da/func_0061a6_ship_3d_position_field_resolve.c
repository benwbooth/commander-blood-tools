#include "../include/bloodprg_ship3d.h"

static volatile cb_u16 CB_FAR *ship_3d_record_word(cb_u16 record_offset,
        cb_u16 field_offset)
{
    return (volatile cb_u16 CB_FAR *)(vm_record_base + record_offset + field_offset);
}

cb_u16 CB_NEAR ship_3d_position_field_resolve(cb_u16 record_offset,
        cb_u16 kind100_compare_word)
{
    volatile bloodprg_vm_object_header CB_FAR *record;
    cb_u16 kind;
    cb_u16 field_offset;
    cb_u16 link_offset;

    record = (volatile bloodprg_vm_object_header CB_FAR *)
        (vm_record_base + record_offset);
    kind = record->kind;

    for (;;) {
        if (kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
            field_offset = (cb_u16)vm_field_offset(
                SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD, kind);
            if (*ship_3d_record_word(record_offset, field_offset) ==
                    kind100_compare_word) {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH, kind);
            } else {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH, kind);
            }
            return (cb_u16)(record_offset + field_offset);
        }

        if (kind == SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8 ||
                kind == SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10 ||
                kind == SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200) {
            field_offset = (cb_u16)vm_field_offset(
                SHIP_3D_FIELD_SELECTOR_POSITION, kind);
            return (cb_u16)(record_offset + field_offset);
        }

        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_PARENT_LINK, kind);
        link_offset = *ship_3d_record_word(record_offset, field_offset);
        if (link_offset == 0xffffu) {
            link_offset = vm_arche_record_offset;
        }

        record_offset = link_offset;
        record = (volatile bloodprg_vm_object_header CB_FAR *)
            (vm_record_base + record_offset);
        kind = record->kind;
    }
}
