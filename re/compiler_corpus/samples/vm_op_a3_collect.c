/* Codegen probe for BLOODPRG 0x005AFD. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define CODE_SEGMENT_TYPE u16
#define CODE_SEGMENT(pointer) FP_SEG(pointer)
#define CODE_BYTE_AT(segment, offset) \
    ((const volatile u8 FAR *)MK_FP((segment), (offset)))
#define CODE_WORD_AT(segment, offset) \
    ((const volatile u16 FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#define NEAR
#define CODE_SEGMENT_TYPE const volatile u8 FAR *
#define CODE_SEGMENT(pointer) (pointer)
#define CODE_BYTE_AT(segment, offset) ((segment) + (offset))
#define CODE_WORD_AT(segment, offset) \
    ((const volatile u16 FAR *)((segment) + (offset)))
#endif

extern volatile u8 FAR *code_image;
extern volatile u16 program_counter;
extern volatile u16 deferred_word;
extern volatile u16 output_words[];

#if defined(__WATCOMC__)
#pragma aux vm_op_a3_collect_probe modify exact []
#endif

void NEAR vm_op_a3_collect_probe(void)
{
    volatile u16 *output;
    CODE_SEGMENT_TYPE code_segment;
    u16 code_offset;
    u16 word;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    code_segment = CODE_SEGMENT(code_image);
    code_offset = program_counter;
    if (*CODE_BYTE_AT(code_segment, code_offset) != 0xa3u) {
#if defined(__WATCOMC__)
        _asm pop es;
        _asm pop ax;
#endif
        return;
    }

    code_offset = (u16)(code_offset + 1u);
    output = output_words;
    for (;;) {
        word = *CODE_WORD_AT(code_segment, code_offset);
        code_offset = (u16)(code_offset + 2u);
        if (word == 0) {
            break;
        }
        *output++ = word;
    }

    word = deferred_word;
    if (word != 0) {
        *output++ = word;
        deferred_word = 0;
    }
    *output = 0;

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
