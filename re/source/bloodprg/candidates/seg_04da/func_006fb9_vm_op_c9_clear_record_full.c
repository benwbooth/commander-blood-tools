#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_c9_clear_record_full(const cb_u8 **script_bytes)
{
    const cb_u16 *script_words;
    cb_u16 record_offset;
    cb_u16 old_kind;
    cb_u16 related_offset;
    cb_u16 reciprocal_offset;
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile bloodprg_vm_record_triple CB_FAR *related;
    volatile bloodprg_vm_record_triple CB_FAR *reciprocal;

    script_words = (const cb_u16 *)*script_bytes;
    record_offset = *script_words++;
    *script_bytes = (const cb_u8 *)script_words;

    record = (volatile bloodprg_vm_record_triple CB_FAR *)
        (vm_record_base + record_offset);
    old_kind = record->kind;
    related_offset = record->related;

    record->kind = 0;
    record->related = 0;
    record->value = 0;

    if (old_kind == BLOODPRG_VM_RECORD_C4) {
        related = (volatile bloodprg_vm_record_triple CB_FAR *)
            (vm_record_base + related_offset);
        reciprocal_offset = (cb_u16)(related_offset +
            (cb_u16)vm_field_offset(BLOODPRG_VM_RECIPROCAL_SELECTOR,
                related->kind));
        reciprocal = (volatile bloodprg_vm_record_triple CB_FAR *)
            (vm_record_base + reciprocal_offset);

        vm_sequence_active = 0;
        vm_ship_3d_depth_step = 6;
        reciprocal->kind = 0;
        reciprocal->related = 0;
        reciprocal->value = 0;
    }
}
