/*
 * Codegen probe for BLOODPRG 0x0060DD.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define KIND_DIRECT_40 0x0040u
#define KIND_100 0x0100u
#define SELECTOR_POSITION 0x000bu
#define SELECTOR_MATCH_WORD 0x000cu
#define SELECTOR_RELATION_WORD 0x000eu
#define SELECTOR_MATCH_POSITION 0x0009u
#define SELECTOR_MISMATCH_POSITION 0x000au

typedef struct object_header {
    u16 kind;
    u8 flags;
} object_header;

typedef struct position_field {
    u16 x;
    u16 y;
} position_field;

extern int NEAR field_offset(u16 selector, u16 kind_mask);
extern volatile position_field NEAR *NEAR position_field_resolve(
        volatile object_header NEAR *record, u16 compare_word);
extern u16 FAR binary_u32_sqrt(u32 value);

#if defined(__WATCOMC__)
#pragma aux field_offset parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux position_field_resolve parm [si] [dx] value [ax] modify exact [ax]
#pragma aux position_distance_probe parm [si] [di] [dx] value [ax] modify exact [ax]
#endif

u16 NEAR position_distance_probe(
        const volatile object_header NEAR *first_record,
        const volatile object_header NEAR *second_record,
        u16 inherited_compare_word)
{
    const volatile u8 NEAR *first_bytes;
    const volatile u8 NEAR *second_bytes;
    const volatile position_field NEAR *first;
    const volatile position_field NEAR *second;
    u16 offset;
    u16 compare_word;
    u16 selector;
    u16 dx_word;
    u16 dy_word;
    i32 dx;
    i32 dy;
    u32 squared;

    first_bytes = (const volatile u8 NEAR *)first_record;
    second_bytes = (const volatile u8 NEAR *)second_record;
    compare_word = inherited_compare_word;

    if (first_record->kind == KIND_100) {
        offset = (u16)field_offset(SELECTOR_RELATION_WORD, second_record->kind);
        compare_word = *(const volatile u16 NEAR *)(second_bytes + offset);
        offset = (u16)field_offset(SELECTOR_MATCH_WORD, first_record->kind);
        selector = *(const volatile u16 NEAR *)(first_bytes + offset) == compare_word
            ? SELECTOR_MATCH_POSITION : SELECTOR_MISMATCH_POSITION;
        offset = (u16)field_offset(selector, first_record->kind);
        first = (const volatile position_field NEAR *)(first_bytes + offset);
    } else if (first_record->kind == KIND_DIRECT_40) {
        offset = (u16)field_offset(SELECTOR_POSITION, first_record->kind);
        first = (const volatile position_field NEAR *)(first_bytes + offset);
    } else {
        first = position_field_resolve(
            (volatile object_header NEAR *)first_record, compare_word);
    }

    if (second_record->kind == KIND_100) {
        offset = (u16)field_offset(SELECTOR_RELATION_WORD, first_record->kind);
        compare_word = *(const volatile u16 NEAR *)(first_bytes + offset);
        offset = (u16)field_offset(SELECTOR_MATCH_WORD, second_record->kind);
        selector = *(const volatile u16 NEAR *)(second_bytes + offset) == compare_word
            ? SELECTOR_MATCH_POSITION : SELECTOR_MISMATCH_POSITION;
        offset = (u16)field_offset(selector, second_record->kind);
        second = (const volatile position_field NEAR *)(second_bytes + offset);
    } else if (second_record->kind == KIND_DIRECT_40) {
        offset = (u16)field_offset(SELECTOR_POSITION, second_record->kind);
        second = (const volatile position_field NEAR *)(second_bytes + offset);
    } else {
        second = position_field_resolve(
            (volatile object_header NEAR *)second_record, compare_word);
    }

    dx_word = (u16)(first->x - second->x);
    if ((dx_word & 0x8000u) != 0) {
        dx_word = (u16)(0u - dx_word);
    }
    dy_word = (u16)(first->y - second->y);
    if ((dy_word & 0x8000u) != 0) {
        dy_word = (u16)(0u - dy_word);
    }

    dx = (i16)dx_word;
    dy = (i16)dy_word;
    squared = (u32)(dx * dx) + (u32)(dy * dy);
    return binary_u32_sqrt(squared);
}
