/*
 * Codegen probe for BLOODPRG 0x006462.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 branch_stack[];
extern volatile u16 branch_stack_top;
extern volatile u8 query_mode;

u16 NEAR vm_branch_stack_return_probe(void)
{
    branch_stack_top = (u16)(branch_stack_top - 2u);
    query_mode = 0;
    return branch_stack[branch_stack_top >> 1];
}
