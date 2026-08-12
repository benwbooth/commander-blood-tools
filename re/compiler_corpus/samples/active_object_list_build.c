/*
 * Codegen probe for BLOODPRG 0x00604E.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define OBJECT_AT(offset) \
    ((const volatile object_header FAR *) \
        MK_FP(FP_SEG(record_base), (offset)))
#else
#define FAR
#define NEAR
#define OBJECT_AT(offset) \
    ((const volatile object_header FAR *)(record_base + (offset)))
#endif

typedef struct directory_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} directory_entry;

typedef struct object_header {
    u16 kind;
    u8 flags;
} object_header;

extern volatile u8 FAR *record_base;
extern const volatile directory_entry FAR *record_directory;
extern volatile u16 active_object_offsets[];

void NEAR active_object_list_probe(void)
{
    const volatile directory_entry FAR *entry;
    const volatile object_header FAR *object;
    volatile u16 *out;

    out = active_object_offsets;
    entry = record_directory;
    while (entry->entry_kind == 1u) {
        object = OBJECT_AT(entry->object_offset);
        if ((object->flags & 2u) != 0) {
            *out = entry->object_offset;
            ++out;
        }
        ++entry;
    }

    *out = 0xffffu;
}
