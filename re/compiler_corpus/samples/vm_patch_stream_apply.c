typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define IMAGE_SEGMENT_TYPE u16
#define IMAGE_SEGMENT(pointer) FP_SEG(pointer)
#define IMAGE_AT(segment, offset) \
    ((volatile u8 FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#define NEAR
#define IMAGE_SEGMENT_TYPE volatile u8 FAR *
#define IMAGE_SEGMENT(pointer) (pointer)
#define IMAGE_AT(segment, offset) ((segment) + (offset))
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define GAME_DATA FAR
#else
#define GAME_DATA
#endif

typedef volatile u8 FAR *far_u8_ptr;

#pragma pack(1)
typedef struct patch_record {
    u16 target_offset;
    u8 value;
} patch_record;
#pragma pack()

extern far_u8_ptr GAME_DATA work_surface;
extern far_u8_ptr GAME_DATA script_image;

u16 NEAR vm_patch_stream_apply_probe(u16 byte_count);

#if defined(__WATCOMC__)
#pragma aux vm_patch_stream_apply_probe parm [ax] value [ax] modify exact [ax]
#endif

u16 NEAR vm_patch_stream_apply_probe(u16 byte_count)
{
    const volatile patch_record FAR *record;
    IMAGE_SEGMENT_TYPE target_segment;
    u16 target_offset;

    record = (const volatile patch_record FAR *)work_surface;
    target_segment = IMAGE_SEGMENT(script_image);

    do {
        target_offset = record->target_offset;
        *IMAGE_AT(target_segment, target_offset) = record->value;
        ++record;
        byte_count = (u16)(byte_count - 3u);
    } while (byte_count != 0);

    return target_offset;
}
