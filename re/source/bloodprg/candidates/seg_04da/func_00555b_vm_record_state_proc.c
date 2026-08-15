#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_RECORD_STATE_OBJECT_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_RECORD_STATE_OBJECT_AT(base, offset) ((base) + (offset))
#endif

void CB_FAR vm_record_state_proc(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    volatile cb_u8 CB_FAR *object;
    volatile cb_u16 *slot;
    cb_i16 field_offset;

    slot = vm_special_slots;
    entry = vm_record_directory_gs;
    while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
        object = VM_RECORD_STATE_OBJECT_AT(
            vm_record_base_gs, entry->object_offset);
        field_offset = (cb_i16)vm_field_offset(
            0x0011u, *(volatile cb_u16 CB_FAR *)object);
        if (*(volatile cb_i16 CB_FAR *)(object + field_offset) == -1) {
            *slot++ = entry->object_offset;
            if ((cb_i16)*slot == -1) {
                break;
            }
        }
        ++entry;
    }
}
