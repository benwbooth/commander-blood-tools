/*
 * Codegen probe for BLOODPRG 0x006596.
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

extern volatile u8 GAME_DATA block_scan_flags;
extern volatile u8 GAME_DATA resume_state;
extern volatile u16 GAME_DATA block_match_value;
extern volatile u16 GAME_DATA resume_value;

const u8 NEAR *NEAR token_special(u16 terminator, const u8 NEAR *script_bytes);
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux token_special parm [ax] [si] value [si] modify exact [si]
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_conditional_block_probe parm [si] value [si] modify exact [ax bp dx si]
#endif

const u8 NEAR *NEAR vm_conditional_block_probe(
        const u8 NEAR *script_bytes)
{
    u8 inverted;
    u16 target;
    u16 match;

    if ((block_scan_flags & 1u) != 0) {
        return token_special(0, script_bytes);
    }

    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    target = *(const u16 NEAR *)script_bytes;
    script_bytes += 2;
    match = ((resume_state & 2u) != 0) ? resume_value : block_match_value;

    if (match != 0) {
        if (inverted != 0) {
            if (target != match) {
                return script_bytes;
            }
        } else if (target == match) {
            return script_bytes;
        }
    }

    return (const u8 NEAR *)vm_branch_probe();
}
