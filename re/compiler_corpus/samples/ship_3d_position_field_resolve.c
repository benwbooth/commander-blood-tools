/*
 * Codegen probe for BLOODPRG 0x0061A6.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#define KIND_DIRECT_8 0x0008u
#define KIND_DIRECT_10 0x0010u
#define KIND_100 0x0100u
#define KIND_DIRECT_200 0x0200u
#define SELECTOR_POSITION 0x000bu
#define SELECTOR_MATCH_WORD 0x000cu
#define SELECTOR_MATCH_POSITION 0x0009u
#define SELECTOR_MISMATCH_POSITION 0x000au
#define SELECTOR_PARENT_LINK 0x0011u

typedef struct object_header {
    u16 kind;
    u8 flags;
} object_header;

typedef struct position_field {
    u16 x;
    u16 y;
} position_field;

extern volatile u16 arche_record_offset;
extern int NEAR field_offset(u16 selector, u16 kind_mask);

#if defined(__WATCOMC__)
#pragma aux field_offset parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux position_field_resolve_probe parm [si] [dx] value [ax] modify exact [ax]
#endif

volatile position_field NEAR *NEAR position_field_resolve_probe(
        volatile object_header NEAR *record, u16 compare_word)
{
    volatile u8 NEAR *record_bytes;
    u16 kind;
    u16 offset;
    u16 link;

    for (;;) {
        record_bytes = (volatile u8 NEAR *)record;
        kind = record->kind;

        if (kind == KIND_100) {
            offset = (u16)field_offset(SELECTOR_MATCH_WORD, kind);
            if (*(volatile u16 NEAR *)(record_bytes + offset) == compare_word) {
                offset = (u16)field_offset(SELECTOR_MATCH_POSITION, kind);
            } else {
                offset = (u16)field_offset(SELECTOR_MISMATCH_POSITION, kind);
            }
            return (volatile position_field NEAR *)(record_bytes + offset);
        }

        if (kind == KIND_DIRECT_8 || kind == KIND_DIRECT_10 ||
                kind == KIND_DIRECT_200) {
            offset = (u16)field_offset(SELECTOR_POSITION, kind);
            return (volatile position_field NEAR *)(record_bytes + offset);
        }

        offset = (u16)field_offset(SELECTOR_PARENT_LINK, kind);
        link = *(volatile u16 NEAR *)(record_bytes + offset);
        if (link == 0xffffu) {
            link = arche_record_offset;
        }
        record = (volatile object_header NEAR *)link;
    }
}
