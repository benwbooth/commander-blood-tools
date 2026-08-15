#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_FLAG_HISTORY_AT(base, offset) \
    ((volatile cb_u16 CB_FAR *)MK_FP( \
        FP_SEG(base), (cb_u16)(FP_OFF(base) + (offset))))
#define VM_FLAG_CODE_AT(base, offset) \
    ((const volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_FLAG_HISTORY_AT(base, offset) \
    ((volatile cb_u16 *)((volatile cb_u8 *)(base) + (offset)))
#define VM_FLAG_CODE_AT(base, offset) ((base) + (offset))
#endif

cb_u16 CB_NEAR vm_flag_test_67b1(void)
{
    bloodprg_vm_image_ptr code_image;
    const volatile bloodprg_value_node CB_FAR *node;
    volatile cb_u16 CB_FAR *history;
    cb_u16 value;
    cb_u16 node_offset;
    cb_u16 payload_offset;
    cb_u16 history_offset;

    value = vm_block_match_value_gs;
    if (value == 0u) {
        return 0u;
    }

    if ((vm_resume_state_gs & BLOODPRG_VM_RESUME_ACTIVE) != 0u) {
        vm_resume_value_gs = value;
    }
    vm_presentation_word_buffer_gs[0] = 0u;

    history = vm_blood_history_words;
    history_offset = vm_blood_history_ring_index;
    *VM_FLAG_HISTORY_AT(history, history_offset) = value;
    vm_blood_history_ring_index =
        (history_offset + 2u) & BLOODPRG_VM_HISTORY_RING_MASK;

    code_image = vm_code_image;
    node_offset = vm_pc_saved;
    while (node_offset != 0u) {
        node = (const volatile bloodprg_value_node CB_FAR *)
            VM_FLAG_CODE_AT(code_image, node_offset);
        if (node->value == value) {
            payload_offset = node_offset + 4u;
            if (*VM_FLAG_CODE_AT(code_image, payload_offset)
                    == BLOODPRG_VM_CONCEPT_OPCODE) {
                vm_branch_b = vm_branch_a;
                vm_parent_program_counter = vm_program_counter;
                vm_branch_a = value;
                vm_program_counter = payload_offset;
            }
            break;
        }
        node_offset = node->next_offset;
    }
    vm_block_match_value_gs = 0u;
    return value;
}
