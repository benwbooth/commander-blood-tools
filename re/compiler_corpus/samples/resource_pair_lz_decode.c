/* Codegen probe for BLOODPRG 0x00AABC. */
typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

const volatile u8 FAR *NEAR resource_pair_lz_decode_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination,
        volatile u8 FAR *destination_end)
{
    const volatile u8 FAR *copy_source;
    u16 distance;
    u16 length;
    u8 control;
    u8 packed;

    for (;;) {
        control = *source++;
        if ((control & 0x80u) == 0u) {
            *destination++ = control == 0u ? 0u : (u8)(control + 12u);
            if (destination >= destination_end) {
                break;
            }
            continue;
        }

        packed = *source++;
        distance = (u16)((((u16)(control & 0x7Fu)) << 1)
                | ((packed >> 4) & 1u));
        ++distance;
        length = (u16)((packed >> 5) + 2u);
        copy_source = destination - distance;
        do {
            *destination++ = *copy_source++;
        } while (--length != 0u);
        if (destination >= destination_end) {
            break;
        }

        for (;;) {
            control = *source++;
            if ((control & 0x80u) != 0u) {
                break;
            }
            *destination++ = control == 0u ? 0u : (u8)(control + 12u);
            if (destination >= destination_end) {
                return source;
            }
        }

        distance = (u16)((((u16)(control & 0x7Fu)) << 1)
                | (packed & 1u));
        ++distance;
        length = (u16)(((packed >> 1) & 7u) + 2u);
        copy_source = destination - distance;
        do {
            *destination++ = *copy_source++;
        } while (--length != 0u);
        if (destination >= destination_end) {
            break;
        }
    }

    return source;
}
