/*
 * Codegen probe for BLOODPRG 0x006559.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 query_mode;
extern volatile u16 branch_stack_top;
extern volatile u16 branch_stack[];

#if defined(__WATCOMC__)
#pragma aux vm_branch_stack_push_probe parm [si] value [si] modify exact [ax bp si]
#endif

const u16 NEAR *NEAR vm_branch_stack_push_probe(
        const u16 NEAR *script_words)
{
    u16 top;
    u16 target;

    query_mode = 1;
    top = branch_stack_top;
    branch_stack_top += 2u;
    target = *script_words++;
    *(volatile u16 NEAR *)
        ((volatile u8 NEAR *)branch_stack + top) = target;

    return script_words;
}
