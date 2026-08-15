/*
 * Codegen probe for BLOODPRG 0x006863.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__WATCOMC__)
#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define FAR far
#define NEAR near
#define GAME_DATA far
#else
#define FAR
#define NEAR
#define GAME_DATA
#endif

extern volatile u8 FAR * GAME_DATA record_base_global;
extern volatile u8 GAME_DATA query_mode;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux vm_shared_state_probe parm [si] value [si] modify exact [ax bx cx dx si es]
#endif

extern u16 NEAR branch_fail_probe(void);

const u8 NEAR *NEAR vm_shared_state_probe(const u8 NEAR *script_bytes)
{
    u16 offset;
    u8 op;
    u8 rhs_mode;
    u16 rhs;
    u16 current;
    volatile u8 FAR *record_base;
    volatile u16 FAR *field;
    u8 pass;

    record_base = record_base_global;
    offset = *(const u16 NEAR *)script_bytes;
    field = (volatile u16 FAR *)(record_base + offset);
    current = *field;
    script_bytes += sizeof(u16);

    op = *script_bytes++;
    rhs_mode = *script_bytes++;
    rhs = *(const u16 NEAR *)script_bytes;
    if (rhs_mode == 0xc0u || rhs_mode == 0xc2u) {
        rhs = *(volatile u16 FAR *)(record_base + rhs);
    }
    script_bytes += sizeof(u16);

    if ((query_mode & 1u) != 0) {
        pass = 0;
        if (op == 0xf0u) {
            pass = current != rhs;
        } else if (op == 0xf3u) {
            pass = (i16)current <= (i16)rhs;
        } else if (op == 0xf4u) {
            pass = (i16)current >= (i16)rhs;
        } else if (op == 0xf1u) {
            pass = (i16)current < (i16)rhs;
        } else if (op == 0xf2u) {
            pass = (i16)current > (i16)rhs;
        } else if (op == 0xf5u) {
            pass = current == rhs;
        }
        if (!pass) {
            return (const u8 NEAR *)branch_fail_probe();
        }
    } else {
        if (op == 0xf6u) {
            current += rhs;
        } else if (op == 0xf7u) {
            current -= rhs;
        } else if (op == 0xf5u) {
            current = rhs;
        }
        *field = current;
    }

    return script_bytes;
}
