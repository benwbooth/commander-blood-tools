/*
 * Codegen probe for BLOODPRG 0x006D80.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define RECORD_AT(base, offset) ((volatile u8 FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define FAR
#define NEAR
#define RECORD_AT(base, offset) ((base) + (offset))
#endif

extern volatile u8 FAR *record_base_global;
extern volatile u8 query_mode;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux vm_c6_record_match_probe parm [si] value [si] modify exact [ax dx si bp es]
#endif

extern u16 NEAR branch_fail_probe(void);

const u8 NEAR *NEAR vm_c6_record_match_probe(const u8 NEAR *script_bytes)
{
    int inverted;
    u16 record_offset;
    u16 operand;
    volatile u8 FAR *record_base;
    volatile u16 FAR *record;
    int matches;

    record_base = record_base_global;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }
    record_offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    operand = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);

    record = (volatile u16 FAR *)RECORD_AT(record_base, record_offset);
    if ((query_mode & 1u) != 0) {
        matches = record[1] == operand && record[0] == 0x00c6u;
        if (matches == inverted) {
            return (const u8 NEAR *)branch_fail_probe();
        }
    } else {
        record[0] = 0x00c6u;
        record[1] = operand;
        record[2] = 0;
    }
    return script_bytes;
}
