/*
 * Codegen probe for BLOODPRG 0x00624B.
 * This is not recovered game source.
 */
#include <dos.h>

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

typedef struct object_header {
    u16 kind;
    u8 flags;
} object_header;

extern const volatile directory_entry FAR *record_directory;
extern int NEAR field_offset(u16 selector, u16 kind_mask);

#define OBJECT_AT(target, offset) \
    ((const volatile object_header FAR *)MK_FP(FP_SEG(target), (offset)))
#define OBJECT_OFFSET(target) ((u16)FP_OFF(target))

#if defined(__WATCOMC__)
#pragma aux field_offset parm [ax] [bx] value [ax] modify exact [ax]
/* Watcom reserves BP; BX stands in for the binary's SS:BP cursor ABI. */
#pragma aux nav_source_list_build_probe parm [es di] [bx] value [bx] modify exact [bx]
#endif

u16 NEAR *FAR nav_source_list_build_probe(
        const volatile object_header FAR *target,
        u16 NEAR *output)
{
    const volatile directory_entry FAR *entry;
    const volatile object_header FAR *object;
    const volatile u16 FAR *parent;
    u16 link_offset;

    entry = record_directory;
    do {
        object = OBJECT_AT(target, entry->object_offset);
        link_offset = (u16)field_offset(0x0011u, object->kind);
        if (link_offset != 0) {
            parent = (const volatile u16 FAR *)
                ((const volatile u8 FAR *)object + link_offset);
            if (*parent == OBJECT_OFFSET(target)) {
                *output = entry->object_offset;
                ++output;
                output = nav_source_list_build_probe(object, output);
            }
        }

        ++entry;
    } while (entry->entry_kind == 1u);

    *output = 0xffffu;
    return output;
}
