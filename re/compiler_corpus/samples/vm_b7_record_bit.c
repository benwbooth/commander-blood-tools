/*
 * Codegen probe for BLOODPRG 0x006AA7.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

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
#pragma aux vm_b7_record_bit_probe parm [si] value [si] modify exact [ax bx cx dx si es]
#endif

extern u16 NEAR branch_fail_probe(void);

const u8 NEAR *NEAR vm_b7_record_bit_probe(const u8 NEAR *script_bytes)
{
    u8 inverted;
    u16 offset;
    u8 bit_index;
    u8 bit_in_byte;
    u8 mask;
    volatile u8 FAR *record_base;
    volatile u8 FAR *field;

    record_base = record_base_global;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    bit_index = *script_bytes++;
    bit_in_byte = (u8)(bit_index & 7u);
    field = record_base + (u16)(offset + (bit_index >> 3));

    if ((query_mode & 1u) != 0) {
        if (((u8)(*field << bit_in_byte) & 0x80u) != 0) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return (const u8 NEAR *)branch_fail_probe();
    }

    mask = (u8)(1u << (7u - bit_in_byte));
    if (!inverted) {
        *field |= mask;
    } else {
        *field &= (u8)~mask;
    }
    return script_bytes;
}
