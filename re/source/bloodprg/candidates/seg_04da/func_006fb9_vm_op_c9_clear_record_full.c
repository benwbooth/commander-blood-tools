#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C9_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C9_RECORD_AT(base, offset) ((base) + (offset))
#endif

const cb_u8 CB_NEAR *CB_NEAR vm_op_c9_clear_record_full(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u16 record_offset;
    cb_u16 old_kind;
    cb_u16 related_offset;
    cb_u16 related_kind;
    cb_u16 reciprocal_offset;
    volatile cb_u8 CB_FAR *record_base;
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile bloodprg_vm_record_triple CB_FAR *related;
    volatile bloodprg_vm_record_triple CB_FAR *reciprocal;

    record_base = vm_record_base_gs;
    record_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    record = (volatile bloodprg_vm_record_triple CB_FAR *)
        VM_C9_RECORD_AT(record_base, record_offset);
    old_kind = record->kind;
    record->kind = 0;
    related_offset = record->related;
    record->related = 0;
    record->value = 0;

    if (old_kind == BLOODPRG_VM_RECORD_C4) {
        related = (volatile bloodprg_vm_record_triple CB_FAR *)
            VM_C9_RECORD_AT(record_base, related_offset);
        related_kind = related->kind;
        reciprocal_offset = (cb_u16)(related_offset +
            (cb_i16)vm_field_offset(BLOODPRG_VM_RECIPROCAL_SELECTOR,
                related_kind));
        reciprocal = (volatile bloodprg_vm_record_triple CB_FAR *)
            VM_C9_RECORD_AT(record_base, reciprocal_offset);

        vm_sequence_active_gs = 0;
        vm_ship_3d_depth_step_gs = 6;
        reciprocal->kind = 0;
        reciprocal->related = 0;
        reciprocal->value = 0;
    }
    return script_bytes;
}
