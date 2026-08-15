#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define BLOODPRG_VM_OBJECT_AT(offset) \
    ((const volatile bloodprg_vm_object_header CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base_gs), (offset)))
#else
#define BLOODPRG_VM_OBJECT_AT(offset) \
    ((const volatile bloodprg_vm_object_header CB_FAR *) \
        (vm_record_base_gs + (offset)))
#endif

void CB_SAVE_REGS CB_NEAR active_object_list_build(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    const volatile bloodprg_vm_object_header CB_FAR *object;
    volatile cb_u16 CB_GAME_DATA *out;

#if defined(__WATCOMC__)
    /* __saveregs restores segments and general registers except AX. */
    _asm push ax;
#endif

    out = vm_active_object_offsets_gs;
    entry = vm_record_directory_gs;
    while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
        object = BLOODPRG_VM_OBJECT_AT(entry->object_offset);
        if ((object->flags & BLOODPRG_VM_OBJECT_IN_PLAY_FLAG) != 0) {
            *out = entry->object_offset;
            ++out;
        }
        ++entry;
    }

    *out = 0xffffu;

#if defined(__WATCOMC__)
    _asm pop ax;
#endif
}
