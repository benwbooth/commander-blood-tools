#include "../include/bloodprg_ship3d.h"

static volatile cb_u16 CB_FAR *ship_3d_record_word(cb_u16 record_offset,
        cb_u16 field_offset)
{
    return (volatile cb_u16 CB_FAR *)(vm_record_base + record_offset + field_offset);
}

static cb_u16 ship_3d_kind100_relation_word(cb_u16 record_offset)
{
    volatile bloodprg_vm_object_header CB_FAR *record;
    cb_u16 field_offset;

    record = (volatile bloodprg_vm_object_header CB_FAR *)
        (vm_record_base + record_offset);
    field_offset = (cb_u16)vm_field_offset(
        SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD, record->kind);
    return *ship_3d_record_word(record_offset, field_offset);
}

static cb_u16 ship_3d_distance_field(cb_u16 record_offset,
        cb_u16 other_record_offset, cb_u16 *kind100_compare_word)
{
    volatile bloodprg_vm_object_header CB_FAR *record;
    cb_u16 field_offset;

    record = (volatile bloodprg_vm_object_header CB_FAR *)
        (vm_record_base + record_offset);
    if (record->kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
        *kind100_compare_word = ship_3d_kind100_relation_word(other_record_offset);
        return ship_3d_position_field_resolve(record_offset, *kind100_compare_word);
    }

    if (record->kind == SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40) {
        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_POSITION, record->kind);
        return (cb_u16)(record_offset + field_offset);
    }

    return ship_3d_position_field_resolve(record_offset, *kind100_compare_word);
}

static cb_u16 ship_3d_abs_word_delta(cb_u16 lhs, cb_u16 rhs)
{
    cb_u16 delta;

    delta = (cb_u16)(lhs - rhs);
    if ((delta & 0x8000u) != 0) {
        delta = (cb_u16)(0u - delta);
    }
    return delta;
}

cb_u16 CB_NEAR ship_3d_position_distance(cb_u16 first_record_offset,
        cb_u16 second_record_offset, cb_u16 inherited_kind100_compare_word)
{
    cb_u16 first_field_offset;
    cb_u16 second_field_offset;
    volatile ship_3d_position_field CB_FAR *first;
    volatile ship_3d_position_field CB_FAR *second;
    cb_i32 dx;
    cb_i32 dy;
    cb_u32 squared;
    cb_u16 kind100_compare_word;

    kind100_compare_word = inherited_kind100_compare_word;
    first_field_offset = ship_3d_distance_field(first_record_offset,
        second_record_offset, &kind100_compare_word);
    second_field_offset = ship_3d_distance_field(second_record_offset,
        first_record_offset, &kind100_compare_word);

    first = (volatile ship_3d_position_field CB_FAR *)
        (vm_record_base + first_field_offset);
    second = (volatile ship_3d_position_field CB_FAR *)
        (vm_record_base + second_field_offset);

    dx = (cb_i16)ship_3d_abs_word_delta(first->x, second->x);
    dy = (cb_i16)ship_3d_abs_word_delta(first->y, second->y);
    squared = (cb_u32)(dx * dx) + (cb_u32)(dy * dy);
    return binary_u32_sqrt(squared);
}
