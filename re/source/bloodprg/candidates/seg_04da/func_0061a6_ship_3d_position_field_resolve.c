#include "../include/bloodprg_ship3d.h"

volatile ship_3d_position_field CB_NEAR *CB_NEAR
ship_3d_position_field_resolve(
        volatile bloodprg_vm_object_header CB_NEAR *record,
        cb_u16 kind100_compare_word)
{
    volatile cb_u8 CB_NEAR *record_bytes;
    cb_u16 kind;
    cb_u16 field_offset;
    cb_u16 link_offset;

    for (;;) {
        record_bytes = (volatile cb_u8 CB_NEAR *)record;
        kind = record->kind;

        if (kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
            field_offset = (cb_u16)vm_field_offset(
                SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD, kind);
            if (*(volatile cb_u16 CB_NEAR *)(record_bytes + field_offset) ==
                    kind100_compare_word) {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_KIND100_POS_MATCH_FIELD, kind);
            } else {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_KIND100_POS_MISMATCH_FIELD, kind);
            }
            return (volatile ship_3d_position_field CB_NEAR *)
                (record_bytes + field_offset);
        }

        if (kind == SHIP_3D_POS_KIND_DIRECT_8 ||
                kind == SHIP_3D_POS_KIND_DIRECT_10 ||
                kind == SHIP_3D_POS_KIND_DIRECT_200) {
            field_offset = (cb_u16)vm_field_offset(
                SHIP_3D_FIELD_SELECTOR_POSITION, kind);
            return (volatile ship_3d_position_field CB_NEAR *)
                (record_bytes + field_offset);
        }

        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_PARENT_LINK, kind);
        link_offset = *(volatile cb_u16 CB_NEAR *)
            (record_bytes + field_offset);
        if (link_offset == 0xffffu) {
            link_offset = vm_arche_record_offset;
        }

        record = (volatile bloodprg_vm_object_header CB_NEAR *)link_offset;
    }
}
