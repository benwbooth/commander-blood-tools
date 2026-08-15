/*
 * Codegen probe for BLOODPRG 0x006462.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__WATCOMC__)
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define NEAR near
#define GAME_DATA far
#else
#define NEAR
#define GAME_DATA
#endif

extern volatile u16 GAME_DATA branch_stack[];
extern volatile u16 GAME_DATA branch_stack_top;
extern volatile u8 GAME_DATA query_mode;

#if defined(__WATCOMC__)
#pragma aux vm_branch_stack_return_probe value [si] modify exact [ax si]
#endif

u16 NEAR vm_branch_stack_return_probe(void)
{
    u16 target;

    branch_stack_top = (u16)(branch_stack_top - 2u);
    target = *(volatile u16 GAME_DATA *)
        ((volatile u8 GAME_DATA *)branch_stack + branch_stack_top);
    query_mode = 0;
    return target;
}
