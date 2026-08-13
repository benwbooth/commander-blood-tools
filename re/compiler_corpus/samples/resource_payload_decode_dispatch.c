/* Codegen probe for BLOODPRG 0x00A82C. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))

typedef struct decode_result {
    const volatile u8 FAR *source;
    volatile u8 FAR *destination;
} decode_result;

extern volatile u16 GAME_DATA decode_mode;

void NEAR decode_ab(const volatile u8 FAR *source,
        volatile u8 FAR *destination);
void NEAR decode_ad(const volatile u8 FAR *source,
        volatile u8 FAR *destination);

decode_result NEAR resource_payload_decode_dispatch_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination,
        u16 alternate_destination_segment)
{
    decode_result result;
    u8 checksum;
    u16 index;

    destination = (volatile u8 FAR *)MK_FP(
            FP_SEG(destination), FP_OFF(destination) & 0xFDFFu);
    checksum = 0u;
    for (index = 0u; index < 6u; ++index) {
        checksum = (u8)(checksum + source[index]);
    }

    if (checksum == 0xADu) {
        decode_mode = 3u;
        destination = (volatile u8 FAR *)MK_FP(
                alternate_destination_segment, FP_OFF(destination));
        decode_ad(source, destination);
        source = (const volatile u8 FAR *)MK_FP(FP_SEG(destination), 0u);
    } else if (checksum == 0xABu) {
        decode_ab(source, destination);
        source = (const volatile u8 FAR *)MK_FP(
                FP_SEG(source), FP_OFF(destination));
    }

    result.source = source;
    result.destination = destination;
    return result;
}
