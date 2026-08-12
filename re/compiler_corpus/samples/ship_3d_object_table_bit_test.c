/*
 * Codegen probe for BLOODPRG 0x006210.
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

typedef struct directory_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} directory_entry;

extern const volatile directory_entry FAR *record_directory;
extern int NEAR field_offset(u16 selector, u16 kind_mask);

#if defined(__WATCOMC__)
#pragma aux field_offset parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux object_table_bit_test_probe parm [ax] [si] value [ax] modify exact [ax]
#endif

int NEAR object_table_bit_test_probe(u16 object_offset,
        const volatile u8 NEAR *bitset_base)
{
    const volatile directory_entry FAR *entry;
    u16 object_index;
    u16 byte_offset;
    u16 shifted;

    entry = record_directory;
    object_index = 0;
    while (entry->object_offset != object_offset) {
        ++entry;
        ++object_index;
    }

    byte_offset = (u16)((u16)field_offset(5u, 2u) + (object_index >> 3));
    shifted = (u16)bitset_base[byte_offset];
    shifted <<= (u16)((object_index & 7u) + 1u);
    return (shifted & 0x0100u) != 0;
}
