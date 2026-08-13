typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define SCRIPT_SEGMENT_TYPE u16
#define SCRIPT_SEGMENT(pointer) FP_SEG(pointer)
#define SCRIPT_AT(segment, offset) \
    ((const volatile u8 FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#define NEAR
#define SCRIPT_SEGMENT_TYPE const volatile u8 FAR *
#define SCRIPT_SEGMENT(pointer) (pointer)
#define SCRIPT_AT(segment, offset) ((segment) + (offset))
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define GAME_DATA FAR
#else
#define GAME_DATA
#endif

typedef volatile u8 FAR *far_u8_ptr;

typedef struct directory_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} directory_entry;

typedef const volatile directory_entry FAR *directory_ptr;

#pragma pack(1)
typedef struct patch_record {
    u16 target_offset;
    u8 value;
} patch_record;
#pragma pack()

extern far_u8_ptr GAME_DATA work_surface;
extern far_u8_ptr GAME_DATA script_image;
extern directory_ptr GAME_DATA record_directory;

u16 NEAR vm_patch_stream_build_probe(void);

#if defined(__WATCOMC__)
#pragma aux vm_patch_stream_build_probe value [ax] modify exact [ax]
#endif

u16 NEAR vm_patch_stream_build_probe(void)
{
    const volatile directory_entry FAR *entry;
    volatile patch_record FAR *record;
    SCRIPT_SEGMENT_TYPE script_segment;
    u16 byte_count;
    u16 object_offset;

    record = (volatile patch_record FAR *)work_surface;
    entry = record_directory;
    script_segment = SCRIPT_SEGMENT(script_image);
    byte_count = 0;

    while (entry->object_offset != 0xffffu) {
        if (entry->entry_kind == 2u) {
            object_offset = entry->object_offset;
            record->target_offset = object_offset;
            record->value = *SCRIPT_AT(script_segment, object_offset);
            ++record;
            byte_count = (u16)(byte_count + (u16)sizeof(patch_record));
        }
        ++entry;
    }

    return byte_count;
}
