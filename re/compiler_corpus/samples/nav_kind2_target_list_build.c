/* Codegen probe for BLOODPRG 0x0071CF. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct vm_object_header_probe {
    u16 kind;
    u8 flags;
} vm_object_header_probe;

extern volatile u8 FAR *record_base_probe;
extern volatile u16 honk_record_probe;
extern volatile u16 menu_record_probe;
extern volatile u16 active_offsets_probe[];
extern volatile u16 target_offsets_probe[];
extern void active_object_list_build_probe(void);

#if defined(__WATCOMC__)
#pragma aux active_object_list_build_probe modify exact []
#pragma aux nav_kind2_target_list_build_probe value [ax] modify exact [ax cx]
#endif

u16 FAR nav_kind2_target_list_build_probe(void)
{
    const volatile u16 *source;
    volatile u16 *destination;
    const volatile vm_object_header_probe FAR *object;
    u16 object_offset;
    u16 count;

    count = 0;
    active_object_list_build_probe();
    source = active_offsets_probe;
    destination = target_offsets_probe;

    for (;;) {
        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
        if (object_offset == honk_record_probe ||
                object_offset == menu_record_probe) {
            continue;
        }

        object = (const volatile vm_object_header_probe FAR *)
            (record_base_probe + object_offset);
        if (object->kind == 2u) {
            *destination++ = object_offset;
            ++count;
        }
    }

    *destination = 0xffffu;
    return count;
}
