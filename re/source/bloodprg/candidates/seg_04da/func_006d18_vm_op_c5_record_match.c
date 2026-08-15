#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C5_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C5_RECORD_AT(base, offset) ((base) + (offset))
#endif

const cb_u8 CB_NEAR *CB_NEAR vm_op_c5_record_match(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 inverted;
    cb_u16 record_offset;
    cb_u16 operand;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *record;
    volatile cb_u16 CB_FAR *related;

    record_base = vm_record_base_gs;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    record_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    operand = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    record = (volatile cb_u16 CB_FAR *)VM_C5_RECORD_AT(
        record_base, record_offset);
    if ((vm_query_mode_gs & 1u) != 0) {
        if (record[1] == operand && record[0] == 0x00c5u) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return (const cb_u8 CB_NEAR *)vm_branch_fail();
    }

    related = (volatile cb_u16 CB_FAR *)VM_C5_RECORD_AT(
        record_base, operand);
    if ((*((volatile cb_u8 CB_FAR *)related + 2) & 1u) == 0
            || related[0] != 0x0200u
            || record[0] != 0) {
        return (const cb_u8 CB_NEAR *)vm_branch_fail();
    }
    record[0] = 0x00c5u;
    record[1] = operand;
    record[2] = 0;
    return script_bytes;
}
