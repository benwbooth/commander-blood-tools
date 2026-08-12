typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define HEAP_SEGMENT_TYPE u16
#define HEAP_SEGMENT(pointer) FP_SEG(pointer)
#define OBJECT_AT(segment, offset) \
    ((volatile object_record FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#define NEAR
#define HEAP_SEGMENT_TYPE volatile u8 FAR *
#define HEAP_SEGMENT(pointer) (pointer)
#define OBJECT_AT(segment, offset) \
    ((volatile object_record FAR *)((segment) + (offset)))
#endif

typedef struct directory_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} directory_entry;

typedef struct object_record {
    u16 kind;
    u8 flags;
    u8 reserved_03[17];
    u8 access_count;
} object_record;

extern volatile u8 FAR *record_base;
extern const volatile directory_entry FAR *record_directory;

void NEAR object_heap_access_probe(void);

#if defined(__WATCOMC__)
#pragma aux object_heap_access_probe modify exact []
#endif

void NEAR object_heap_access_probe(void)
{
    const volatile directory_entry FAR *entry;
    volatile object_record FAR *object;
    HEAP_SEGMENT_TYPE object_segment;

#if defined(__WATCOMC__)
    /* Preserve the recovered no-clobber call boundary. */
    _asm push ax;
    _asm push es;
#endif

    object_segment = HEAP_SEGMENT(record_base);
    entry = record_directory;
    do {
        object = OBJECT_AT(object_segment, entry->object_offset);
        if ((object->kind & 0x0118u) != 0 &&
                (object->flags & 0x02u) != 0) {
            ++object->access_count;
        }
        ++entry;
    } while (entry->entry_kind == 1u);

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
