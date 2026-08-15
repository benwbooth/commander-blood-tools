#include "../include/bloodprg_ship3d.h"

cb_u16 CB_NEAR ship_3d_position_distance(
        const volatile bloodprg_vm_object_header CB_NEAR *first_record,
        const volatile bloodprg_vm_object_header CB_NEAR *second_record,
        cb_u16 inherited_kind100_compare_word)
{
    const volatile cb_u8 CB_NEAR *first_record_bytes;
    const volatile cb_u8 CB_NEAR *second_record_bytes;
    const volatile ship_3d_position_field CB_NEAR *first;
    const volatile ship_3d_position_field CB_NEAR *second;
    cb_u16 field_offset;
    cb_u16 compare_word;
    cb_u16 selector;
    cb_u16 dx_word;
    cb_u16 dy_word;
    cb_i32 dx;
    cb_i32 dy;
    cb_u32 squared;

    first_record_bytes = (const volatile cb_u8 CB_NEAR *)first_record;
    second_record_bytes = (const volatile cb_u8 CB_NEAR *)second_record;
    compare_word = inherited_kind100_compare_word;

    if (first_record->kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD, second_record->kind);
        compare_word = *(const volatile cb_u16 CB_NEAR *)
            (second_record_bytes + field_offset);
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD, first_record->kind);
        selector = *(const volatile cb_u16 CB_NEAR *)
            (first_record_bytes + field_offset) == compare_word
            ? SHIP_3D_KIND100_POS_MATCH_FIELD
            : SHIP_3D_KIND100_POS_MISMATCH_FIELD;
        field_offset = (cb_u16)vm_field_offset(selector, first_record->kind);
        first = (const volatile ship_3d_position_field CB_NEAR *)
            (first_record_bytes + field_offset);
    } else if (first_record->kind == SHIP_3D_POS_KIND_DIRECT_40) {
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_POSITION, first_record->kind);
        first = (const volatile ship_3d_position_field CB_NEAR *)
            (first_record_bytes + field_offset);
    } else {
        first = ship_3d_position_field_resolve(
            (volatile bloodprg_vm_object_header CB_NEAR *)first_record,
            compare_word);
    }

    if (second_record->kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD, first_record->kind);
        compare_word = *(const volatile cb_u16 CB_NEAR *)
            (first_record_bytes + field_offset);
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD, second_record->kind);
        selector = *(const volatile cb_u16 CB_NEAR *)
            (second_record_bytes + field_offset) == compare_word
            ? SHIP_3D_KIND100_POS_MATCH_FIELD
            : SHIP_3D_KIND100_POS_MISMATCH_FIELD;
        field_offset = (cb_u16)vm_field_offset(selector, second_record->kind);
        second = (const volatile ship_3d_position_field CB_NEAR *)
            (second_record_bytes + field_offset);
    } else if (second_record->kind == SHIP_3D_POS_KIND_DIRECT_40) {
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_POSITION, second_record->kind);
        second = (const volatile ship_3d_position_field CB_NEAR *)
            (second_record_bytes + field_offset);
    } else {
        second = ship_3d_position_field_resolve(
            (volatile bloodprg_vm_object_header CB_NEAR *)second_record,
            compare_word);
    }

    dx_word = (cb_u16)(first->x - second->x);
    if ((dx_word & 0x8000u) != 0) {
        dx_word = (cb_u16)(0u - dx_word);
    }
    dy_word = (cb_u16)(first->y - second->y);
    if ((dy_word & 0x8000u) != 0) {
        dy_word = (cb_u16)(0u - dy_word);
    }

    dx = (cb_i16)dx_word;
    dy = (cb_i16)dy_word;
    squared = (cb_u32)(dx * dx) + (cb_u32)(dy * dy);
    return binary_u32_sqrt(squared);
}
