/* Codegen probe for BLOODPRG 0x00721A. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct object_header {
    u16 kind;
    u8 flags;
} object_header;

extern volatile u8 FAR *record_base;
extern volatile u16 active_object_offsets[];
extern volatile u16 nav_chart_object_offsets[];

void NEAR active_object_list_probe(void);

#if defined(__WATCOMC__)
#pragma aux active_object_list_probe modify exact []
#pragma aux nav_chart_list_probe value [ax] modify exact [ax]
#endif

u16 FAR nav_chart_list_probe(void)
{
    const volatile u16 *source;
    volatile u16 *destination;
    const volatile object_header FAR *object;
    u16 object_offset;
    u16 count;

    count = 0;
    active_object_list_probe();
    source = active_object_offsets;
    destination = nav_chart_object_offsets;

    for (;;) {
        object_offset = *source++;
        if ((i16)object_offset < 0) {
            break;
        }

        object = (const volatile object_header FAR *)
            (record_base + object_offset);
        if ((object->kind & 0x0118u) != 0) {
            *destination++ = object_offset;
            ++count;
        }
    }

    *destination = object_offset;
    return count;
}
