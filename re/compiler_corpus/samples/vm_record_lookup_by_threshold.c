/*
 * Codegen probe for BLOODPRG 0x006034.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct dir_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} dir_entry;

extern const volatile dir_entry FAR *record_directory;

#if defined(__WATCOMC__)
#pragma aux record_lookup_probe parm [ax] value [ax] modify exact [ax]
#endif

u16 NEAR record_lookup_probe(u16 threshold)
{
    const volatile dir_entry FAR *entry;

    entry = record_directory;
    while (threshold > entry->object_offset) {
        ++entry;
    }
    --entry;

    return entry->object_offset;
}
