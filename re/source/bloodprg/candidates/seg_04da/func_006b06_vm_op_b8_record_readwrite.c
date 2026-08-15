#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_B8_RECORD_OFFSET(base, offset) \
    ((cb_u16)(FP_OFF(base) + (offset)))
#define VM_B8_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_B8_RECORD_OFFSET(base, offset) (offset)
#define VM_B8_RECORD_AT(base, offset) ((base) + (offset))
#endif

const cb_u8 CB_NEAR *CB_NEAR vm_op_b8_record_readwrite(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u16 offset;
    cb_u16 record_offset;
    cb_u16 first;
    cb_u16 second;
    cb_u16 owner;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *field;
    volatile cb_u16 CB_FAR *secondary_link;

    record_base = vm_record_base_gs;
    offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    record_offset = VM_B8_RECORD_OFFSET(record_base, offset);
    first = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    second = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    field = (volatile cb_u16 CB_FAR *)VM_B8_RECORD_AT(
        record_base, record_offset);
    if ((vm_query_mode_gs & 1u) != 0) {
        if (field[0] != first || field[1] != second) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
    } else {
        field[0] = first;
        field[1] = second;
        owner = vm_record_lookup_by_threshold(record_offset);
        secondary_link = (volatile cb_u16 CB_FAR *)VM_B8_RECORD_AT(
            record_base, (cb_u16)(vm_arche_record_offset_gs + 0x16u));
        if (owner == *secondary_link) {
            *secondary_link = 0;
        }
    }
    return script_bytes;
}
