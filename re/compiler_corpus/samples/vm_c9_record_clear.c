/*
 * Codegen probe for BLOODPRG 0x006FB9.
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

#define RECORD_C4 0x00c4u
#define RECIPROCAL_SELECTOR 0x0013u

typedef struct record_triple {
    u16 kind;
    u16 related;
    u16 value;
} record_triple;

extern volatile u8 FAR *record_base;
extern volatile u8 sequence_active;
extern volatile u8 depth_step;
int NEAR field_offset_probe(u16 selector, u16 kind_mask);
#if defined(__WATCOMC__)
#pragma aux field_offset_probe parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux vm_c9_record_clear_probe parm [si] value [si] modify exact [ax bx cx si es]
#endif

const u8 NEAR *NEAR vm_c9_record_clear_probe(const u8 NEAR *script_bytes)
{
    u16 record_offset;
    u16 old_kind;
    u16 related_offset;
    u16 related_kind;
    u16 reciprocal_offset;
    volatile u8 FAR *base;
    volatile record_triple FAR *record;
    volatile record_triple FAR *related;
    volatile record_triple FAR *reciprocal;

    base = record_base;
    record_offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);

    record = (volatile record_triple FAR *)RECORD_AT(base, record_offset);
    old_kind = record->kind;
    record->kind = 0;
    related_offset = record->related;
    record->related = 0;
    record->value = 0;

    if (old_kind == RECORD_C4) {
        related = (volatile record_triple FAR *)RECORD_AT(base, related_offset);
        related_kind = related->kind;
        reciprocal_offset = (u16)(related_offset +
            (int)field_offset_probe(RECIPROCAL_SELECTOR, related_kind));
        reciprocal = (volatile record_triple FAR *)RECORD_AT(base, reciprocal_offset);
        sequence_active = 0;
        depth_step = 6;
        reciprocal->kind = 0;
        reciprocal->related = 0;
        reciprocal->value = 0;
    }
    return script_bytes;
}
