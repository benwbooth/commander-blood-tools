#include "../include/bloodprg_list.h"

#define RESOURCE_PAIR_CONTROL_MATCH 0x80u
#define RESOURCE_PAIR_CONTROL_DISTANCE 0x7Fu
#define RESOURCE_PAIR_LITERAL_BIAS 12u
#define RESOURCE_PAIR_LENGTH_BIAS 2u

const volatile cb_u8 CB_FAR *CB_NEAR resource_pair_lz_decode(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination,
        volatile cb_u8 CB_FAR *destination_end)
{
    const volatile cb_u8 CB_FAR *copy_source;
    cb_u16 distance;
    cb_u16 length;
    cb_u8 control;
    cb_u8 packed;

    for (;;) {
        control = *source++;
        if ((control & RESOURCE_PAIR_CONTROL_MATCH) == 0u) {
            *destination++ = control == 0u
                    ? 0u
                    : (cb_u8)(control + RESOURCE_PAIR_LITERAL_BIAS);
            if (destination >= destination_end) {
                break;
            }
            continue;
        }

        packed = *source++;
        distance = (cb_u16)(
                (((cb_u16)(control & RESOURCE_PAIR_CONTROL_DISTANCE)) << 1)
                | ((packed >> 4) & 1u));
        ++distance;
        length = (cb_u16)((packed >> 5) + RESOURCE_PAIR_LENGTH_BIAS);
        copy_source = destination - distance;
        do {
            *destination++ = *copy_source++;
        } while (--length != 0u);
        if (destination >= destination_end) {
            break;
        }

        for (;;) {
            control = *source++;
            if ((control & RESOURCE_PAIR_CONTROL_MATCH) != 0u) {
                break;
            }
            *destination++ = control == 0u
                    ? 0u
                    : (cb_u8)(control + RESOURCE_PAIR_LITERAL_BIAS);
            if (destination >= destination_end) {
                return source;
            }
        }

        distance = (cb_u16)(
                (((cb_u16)(control & RESOURCE_PAIR_CONTROL_DISTANCE)) << 1)
                | (packed & 1u));
        ++distance;
        length = (cb_u16)(
                ((packed >> 1) & 7u) + RESOURCE_PAIR_LENGTH_BIAS);
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
