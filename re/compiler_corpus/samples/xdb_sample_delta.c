/* Codegen probe for the shared XDB slot-8 sample-delta method. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

typedef struct object_record {
    volatile i16 position;
    u8 field_002[0x12];
} object_record;

typedef struct method_context {
    u8 field_000[0x1c];
    u16 object_offset;
    u16 field_01e;
    u16 object_count;
    u8 field_022[0x16];
    u16 sample_cursor;
    i16 previous_sample;
} method_context;

extern volatile u16 object_segment;
extern volatile u8 motion_samples[];

i16 near xdb_sample_delta_probe(method_context near *context);

#if defined(__WATCOMC__)
#pragma aux xdb_sample_delta_probe \
        parm [di] value [ax] modify exact [ax bx cx si]
#endif

i16 near xdb_sample_delta_probe(method_context near *context)
{
    u16 cursor = context->sample_cursor;
    i16 previous = context->previous_sample;
    i16 current;
    i16 delta;
    object_record far *object;
    u16 count;

    current = *(volatile i16 near *)(motion_samples + cursor);
    context->sample_cursor = (cursor + 4u) & 0x0ffcu;
    context->previous_sample = current;
    delta = (i16)((u16)current - (u16)previous);
    object = (object_record far *)MK_FP(object_segment, context->object_offset);
    count = context->object_count;
    do {
        object->position = (i16)((u16)object->position + (u16)delta);
        ++object;
    } while (--count != 0u);
    return delta;
}
