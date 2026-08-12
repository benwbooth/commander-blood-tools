/*
 * Codegen probe for BLOODPRG 0x006B06.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define RECORD_OFFSET(base, offset) ((u16)(FP_OFF(base) + (offset)))
#define RECORD_AT(base, offset) ((volatile u8 FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define FAR
#define NEAR
#define RECORD_OFFSET(base, offset) (offset)
#define RECORD_AT(base, offset) ((base) + (offset))
#endif

extern volatile u8 FAR *record_base_global;
extern volatile u8 query_mode;
extern volatile u16 secondary_record_offset;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux lookup_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux vm_b8_record_pair_probe parm [si] value [si] modify exact [ax bx si es]
#endif

extern u16 NEAR branch_fail_probe(void);
extern u16 NEAR lookup_owner_probe(u16 threshold);

const u8 NEAR *NEAR vm_b8_record_pair_probe(const u8 NEAR *script_bytes)
{
    u16 offset;
    u16 record_offset;
    u16 first;
    u16 second;
    u16 owner;
    volatile u8 FAR *record_base;
    volatile u16 FAR *field;
    volatile u16 FAR *secondary_link;

    record_base = record_base_global;
    offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    record_offset = RECORD_OFFSET(record_base, offset);
    first = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    second = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);

    field = (volatile u16 FAR *)RECORD_AT(record_base, record_offset);
    if ((query_mode & 1u) != 0) {
        if (field[0] != first || field[1] != second) {
            return (const u8 NEAR *)branch_fail_probe();
        }
    } else {
        field[0] = first;
        field[1] = second;
        owner = lookup_owner_probe(record_offset);
        secondary_link = (volatile u16 FAR *)RECORD_AT(
            record_base, (u16)(secondary_record_offset + 0x16u));
        if (owner == *secondary_link) {
            *secondary_link = 0;
        }
    }
    return script_bytes;
}
