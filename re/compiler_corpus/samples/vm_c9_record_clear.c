/*
 * Codegen probe for BLOODPRG 0x006FB9.
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

void NEAR vm_c9_record_clear_probe(const u8 **script_bytes)
{
    const u16 *script_words;
    u16 record_offset;
    u16 old_kind;
    u16 related_offset;
    u16 reciprocal_offset;
    volatile record_triple FAR *record;
    volatile record_triple FAR *related;
    volatile record_triple FAR *reciprocal;

    script_words = (const u16 *)*script_bytes;
    record_offset = *script_words++;
    *script_bytes = (const u8 *)script_words;

    record = (volatile record_triple FAR *)(record_base + record_offset);
    old_kind = record->kind;
    related_offset = record->related;
    record->kind = 0;
    record->related = 0;
    record->value = 0;

    if (old_kind == RECORD_C4) {
        related = (volatile record_triple FAR *)(record_base + related_offset);
        reciprocal_offset = (u16)(related_offset +
            (u16)field_offset_probe(RECIPROCAL_SELECTOR, related->kind));
        reciprocal = (volatile record_triple FAR *)(record_base + reciprocal_offset);
        sequence_active = 0;
        depth_step = 6;
        reciprocal->kind = 0;
        reciprocal->related = 0;
        reciprocal->value = 0;
    }
}
