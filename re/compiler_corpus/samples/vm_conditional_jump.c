/*
 * Codegen probe for BLOODPRG 0x006830.
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
extern volatile u16 GAME_DATA branch_stack[];
extern volatile u16 GAME_DATA branch_stack_top;

#if defined(__WATCOMC__)
#pragma aux vm_conditional_jump_probe parm [si] value [si] modify exact [ax si]
#endif

const u8 NEAR *NEAR vm_conditional_jump_probe(
        const u8 NEAR *script_bytes)
{
    u8 flags;
    u16 target;

    flags = *script_bytes++;
    if ((flags & 1u) == 0) {
        return (const u8 NEAR *)*(const u16 NEAR *)script_bytes;
    }

    query_mode = 1;
    target = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    branch_stack[0] = target;
    branch_stack_top = 2;
    return script_bytes;
}
