/*
 * Codegen probe for BLOODPRG 0x006902.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 FAR *record_base_global;
extern volatile u8 query_mode;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux vm_shared_bit_state_probe parm [si] value [si] modify exact [ax bx dx si es]
#endif

extern u16 NEAR branch_fail_probe(void);

const u8 NEAR *NEAR vm_shared_bit_state_probe(const u8 NEAR *script_bytes)
{
    int inverted;
    u16 offset;
    u16 mask;
    volatile u8 FAR *record_base;
    volatile u16 FAR *field;
    int has_bits;

    record_base = record_base_global;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    mask = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    field = (volatile u16 FAR *)(record_base + offset);

    if ((query_mode & 1u) != 0) {
        has_bits = ((*field & mask) != 0);
        if (has_bits == inverted) {
            return (const u8 NEAR *)branch_fail_probe();
        }
    } else if (!inverted) {
        *field |= mask;
    } else {
        *field &= (u16)~mask;
    }

    return script_bytes;
}
