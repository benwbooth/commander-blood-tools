/*
 * Codegen probe for BLOODPRG 0x006559.
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

extern volatile u8 GAME_DATA query_mode;
extern volatile u16 GAME_DATA branch_stack_top;
extern volatile u16 GAME_DATA branch_stack[];

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
    *(volatile u16 GAME_DATA *)
        ((volatile u8 GAME_DATA *)branch_stack + top) = target;

    return script_words;
}
