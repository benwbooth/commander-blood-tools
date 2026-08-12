/*
 * Codegen probe for BLOODPRG 0x006572.
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

#if defined(__WATCOMC__)
#pragma aux vm_branch_stack_pop_probe value [ax] modify exact [ax]
#endif

u16 NEAR vm_branch_stack_pop_probe(void)
{
    u16 top;

    query_mode = 0;
    top = branch_stack_top;
    if (top == 2u) {
        return top;
    }

    branch_stack_top -= 2u;
    return top;
}
