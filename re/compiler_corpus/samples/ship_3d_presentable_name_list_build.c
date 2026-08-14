/* Codegen probe for BLOODPRG 0x007259. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct object_header_probe {
    u16 kind;
    u8 flags;
} object_header_probe;

extern volatile u16 source_offsets_probe[];
extern volatile u16 output_name_offsets_probe[];
extern volatile u16 arche_record_probe;
extern u16 NEAR *FAR source_list_build_probe(
        const volatile object_header_probe FAR *target,
        u16 NEAR *output);

#if defined(__WATCOMC__)
#pragma aux source_list_build_probe parm [es di] [bx] value [bx] modify exact [bx]
#pragma aux presentable_name_list_build_probe parm [es di] value [bp] modify exact [bp]
#endif

volatile u16 NEAR *FAR presentable_name_list_build_probe(
        const volatile object_header_probe FAR *target)
{
    const volatile u16 *source;
    volatile u16 *destination;
    const volatile object_header_probe FAR *object;
    u16 object_offset;

    source_list_build_probe(target, (u16 NEAR *)source_offsets_probe);
    source = source_offsets_probe;
    destination = output_name_offsets_probe;
    object_offset = (u16)FP_OFF(target);

    for (;;) {
        object = (const volatile object_header_probe FAR *)
            MK_FP(FP_SEG(target), object_offset);
        if ((object->kind & 0x0098u) != 0u &&
                (object->flags & 0x02u) != 0u &&
                object_offset != arche_record_probe) {
            *destination++ = (u16)(object_offset + 4u);
        }

        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
    }

    *destination = 0xffffu;
    return destination;
}
