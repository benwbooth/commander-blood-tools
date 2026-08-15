#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_CODE_SEGMENT_TYPE cb_u16
#define VM_CODE_SEGMENT(pointer) FP_SEG(pointer)
#define VM_CODE_BYTE_AT(segment, offset) \
    ((const volatile cb_u8 CB_FAR *)MK_FP((segment), (offset)))
#define VM_CODE_WORD_AT(segment, offset) \
    ((const volatile cb_u16 CB_FAR *)MK_FP((segment), (offset)))
#else
#define VM_CODE_SEGMENT_TYPE const volatile cb_u8 CB_FAR *
#define VM_CODE_SEGMENT(pointer) (pointer)
#define VM_CODE_BYTE_AT(segment, offset) ((segment) + (offset))
#define VM_CODE_WORD_AT(segment, offset) \
    ((const volatile cb_u16 CB_FAR *)((segment) + (offset)))
#endif

void CB_NEAR vm_op_a3_collect(void)
{
    volatile cb_u16 CB_GAME_DATA *output;
    VM_CODE_SEGMENT_TYPE code_segment;
    cb_u16 code_offset;
    cb_u16 word;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    code_segment = VM_CODE_SEGMENT(vm_code_image);
    code_offset = vm_program_counter;
    if (*VM_CODE_BYTE_AT(code_segment, code_offset)
            != BLOODPRG_VM_CONCEPT_OPCODE) {
#if defined(__WATCOMC__)
        _asm pop es;
        _asm pop ax;
#endif
        return;
    }

    code_offset = (cb_u16)(code_offset + 1u);
    output = vm_presentation_word_buffer_gs;
    for (;;) {
        word = *VM_CODE_WORD_AT(code_segment, code_offset);
        code_offset = (cb_u16)(code_offset + 2u);
        if (word == 0) {
            break;
        }
        *output++ = word;
    }

    word = vm_presentation_reg_6770_gs;
    if (word != 0) {
        *output++ = word;
        vm_presentation_reg_6770_gs = 0;
    }
    *output = 0;

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
