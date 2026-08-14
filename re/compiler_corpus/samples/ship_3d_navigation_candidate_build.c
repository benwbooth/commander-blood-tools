/* Codegen probe for BLOODPRG 0x0070EE. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct vm_object_header_probe {
    u16 kind;
    u8 flags;
} vm_object_header_probe;

extern volatile u8 FAR *record_base_probe;
extern volatile u16 honk_record_probe;
extern volatile u16 source_offsets_probe[];
extern volatile u16 candidate_offsets_probe[];
extern u16 NEAR *FAR source_list_build_probe(
        const volatile vm_object_header_probe FAR *target,
        u16 NEAR *output);

#if defined(__WATCOMC__)
#pragma aux source_list_build_probe parm [es di] [bx] value [bx] modify exact [bx]
#pragma aux navigation_candidate_build_probe parm [es di] modify exact [bx di es]
#endif

void FAR navigation_candidate_build_probe(
        const volatile vm_object_header_probe FAR *target)
{
    const volatile u16 *source;
    volatile u16 *destination;
    const volatile vm_object_header_probe FAR *object;
    u16 object_offset;

    source_list_build_probe(target, (u16 NEAR *)source_offsets_probe);
    source = source_offsets_probe;
    destination = candidate_offsets_probe;

    for (;;) {
        object_offset = *source++;
        if (object_offset == 0xffffu) {
            break;
        }
        if (object_offset == honk_record_probe) {
            continue;
        }

        object = (const volatile vm_object_header_probe FAR *)
            (record_base_probe + object_offset);
        if (object->kind == 2u && (object->flags & 1u) != 0u) {
            *destination++ = object_offset;
        }
    }

    *destination = 0;
}
