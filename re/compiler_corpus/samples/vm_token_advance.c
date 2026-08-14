/*
 * Codegen probe for BLOODPRG 0x0062B6.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile u8 FAR *image_ptr;

typedef struct opcode_descriptor_probe {
    u8 mode_zero_length;
    i8 mode_one_length_or_control;
} opcode_descriptor_probe;

extern volatile u8 GAME_DATA vm_query_mode_probe;
extern volatile u8 GAME_DATA vm_block_scan_flags_probe;
extern const opcode_descriptor_probe GAME_DATA vm_opcode_descriptors_probe[0x60];

const u8 NEAR *NEAR vm_token_special_probe(u16 terminator,
        const u8 NEAR *script_bytes);

#if defined(__WATCOMC__)
#pragma aux vm_token_advance_probe parm [ds si] value [ds si] modify exact [si]
#pragma aux vm_token_special_probe parm [ax] [si] value [si] modify exact [si]
#endif

image_ptr NEAR vm_token_advance_probe(image_ptr script_bytes)
{
    const opcode_descriptor_probe GAME_DATA *descriptor;
    const u8 GAME_DATA *mode_lengths;
    const u8 NEAR *next_script_bytes;
    u8 opcode;
    u8 length;
    i8 control;

    opcode = *script_bytes++;
    descriptor = vm_opcode_descriptors_probe + (i8)(opcode - 0xa0u);
    control = descriptor->mode_one_length_or_control;

    if (control >= 0) {
        mode_lengths = (const u8 GAME_DATA *)descriptor;
        length = mode_lengths[vm_query_mode_probe];
    } else if (control == -1) {
        vm_query_mode_probe = 1;
        length = descriptor->mode_zero_length;
    } else if (control == -2) {
        vm_query_mode_probe = 0;
        length = descriptor->mode_zero_length;
    } else if (control == -3) {
        if (*script_bytes == 0xa1u) {
            ++script_bytes;
        }
        length = descriptor->mode_zero_length;
    } else if ((vm_block_scan_flags_probe & 1u) != 0) {
        length = 0;
    } else {
        if (control == -5 && *script_bytes == 0xa1u) {
            ++script_bytes;
        }
        length = descriptor->mode_zero_length;
    }

    if (length != 0) {
        return script_bytes + (i8)(u8)(length - 1u);
    }

    if (opcode == 0xa6u) {
        script_bytes += 5;
        while (*(const volatile u16 FAR *)script_bytes != 0) {
            script_bytes += 2;
        }
        return script_bytes + 2;
    }

    next_script_bytes = vm_token_special_probe(
            0, (const u8 NEAR *)script_bytes);
    return (image_ptr)MK_FP(
            FP_SEG(script_bytes), (u16)next_script_bytes);
}
