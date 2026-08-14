/* Codegen probe for BLOODPRG 0x00739B. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef volatile u8 FAR *image_ptr;

extern image_ptr script_image_probe;
extern image_ptr code_image_probe;
extern image_ptr record_image_probe;
extern volatile u16 query_mode_word_probe;
extern volatile u8 block_scan_flags_probe;

extern int NEAR field_offset_probe(u16 selector, u16 kind_mask);
extern image_ptr NEAR token_advance_probe(image_ptr script_bytes);

#if defined(__WATCOMC__)
#pragma aux field_offset_probe parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux token_advance_probe parm [ds si] value [ds si] modify exact [si]
#pragma aux vm_cod_scan_probe parm [bx] value [bx] modify exact [bx]
#endif

u16 NEAR vm_cod_scan_probe(u16 object_offset)
{
    image_ptr token;
    image_ptr record;
    u16 saved_query_mode;
    u16 kind;
    u16 code_offset;
    u8 opcode;

    saved_query_mode = query_mode_word_probe;
    token = script_image_probe;
    for (;;) {
        opcode = *token;
        if (opcode == 0xffu) {
            break;
        }
        if (opcode == 0xa6u &&
                *(volatile u16 FAR *)(token + 1u) == object_offset) {
            token[5] |= 0x80u;
        }
        token = token_advance_probe(token);
    }

    block_scan_flags_probe = 1u;
    record = record_image_probe + object_offset;
    token = code_image_probe;
    kind = *(volatile u16 FAR *)record;
    record += field_offset_probe(2u, kind);
    code_offset = *(volatile u16 FAR *)record;

    if (code_offset != 0u) {
        token += code_offset;
        for (;;) {
            opcode = *token;
            if (opcode == 0xffu || opcode == 0xaau) {
                break;
            }
            if (opcode == 0xa6u) {
                token[5] |= 0x80u;
            }
            token = token_advance_probe(token);
        }
    }

    query_mode_word_probe = saved_query_mode;
    return kind;
}
