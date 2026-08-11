#include "bloodprg/vm_records.h"

static cb_u16 cb_read16_far(const cb_u8 CB_FAR *p)
{
    return (cb_u16)(p[0] | ((cb_u16)p[1] << 8));
}
static void cb_write16_far(cb_u8 CB_FAR *p, cb_u16 value)
{
    p[0] = (cb_u8)(value & 0xffu);
    p[1] = (cb_u8)((value >> 8) & 0xffu);
}

static cb_i16 cb_vm_field_offset_006023(
    const cb_i8 CB_FAR *selector_field_offsets,
    cb_u16 selector,
    cb_u16 kind_mask)
{
    cb_u16 bit_index;

    bit_index = 0;
    while ((kind_mask & (cb_u16)(1u << bit_index)) == 0) {
        ++bit_index;
    }

    return (cb_i16)selector_field_offsets[(cb_u16)((selector << 4) + bit_index)];
}

/*
 * BLOODPRG 0x006FB9.
 *
 * Assembly source:
 * re/assembly/bloodprg/seg_04da/func_006fb9_vm_op_c9_clear_record_full.asm
 *
 * Clears the three-word record selected by the VM operand. If the old record
 * type was 0x00c4, the routine follows the related record pointer and clears
 * the reciprocal selector-0x13 three-word field as well.
 */
void CB_NEAR cb_bloodprg_006fb9_vm_op_c9_clear_record_full(
    cb_u8 CB_FAR *record_heap,
    cb_u16 record_off,
    const cb_i8 CB_FAR *selector_field_offsets,
    cb_u8 CB_FAR *nav_state_252a,
    cb_u8 CB_FAR *nav_state_2531)
{
    cb_u16 old_type;
    cb_u16 related_off;
    cb_u16 related_kind;
    cb_i16 reciprocal_delta;
    cb_u16 reciprocal_off;

    old_type = cb_read16_far(record_heap + record_off);
    cb_write16_far(record_heap + record_off, 0);

    related_off = cb_read16_far(record_heap + (cb_u16)(record_off + 2u));
    cb_write16_far(record_heap + (cb_u16)(record_off + 2u), 0);
    cb_write16_far(record_heap + (cb_u16)(record_off + 4u), 0);

    if (old_type == 0x00c4u) {
        related_kind = cb_read16_far(record_heap + related_off);
        reciprocal_delta =
            cb_vm_field_offset_006023(selector_field_offsets, 0x0013u, related_kind);
        reciprocal_off = (cb_u16)(related_off + reciprocal_delta);

        *nav_state_252a = 0;
        *nav_state_2531 = 6;

        cb_write16_far(record_heap + reciprocal_off, 0);
        cb_write16_far(record_heap + (cb_u16)(reciprocal_off + 2u), 0);
        cb_write16_far(record_heap + (cb_u16)(reciprocal_off + 4u), 0);
    }
}
