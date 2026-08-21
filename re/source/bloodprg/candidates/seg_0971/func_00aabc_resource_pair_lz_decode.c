#include "../include/bloodprg_list.h"

#define RESOURCE_PAIR_CONTROL_DISTANCE 0x7Fu
#define RESOURCE_PAIR_LENGTH_BIAS 2u

const volatile cb_u8 CB_FAR *CB_NEAR resource_pair_lz_decode(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination,
        volatile cb_u8 CB_FAR *destination_end,
        cb_u8 literal_bias)
{
    const volatile cb_u8 CB_FAR *copy_source;
    volatile cb_u8 CB_FAR *output;
    volatile cb_u8 CB_FAR *output_end;
    cb_u16 length;
    cb_u8 control;
    cb_u8 packed;

    output = destination;
    output_end = destination_end;

    for (;;) {
        control = *source++;
        if ((cb_i8)control >= 0) {
            *output++ = control == 0u
                    ? 0u
                    : (cb_u8)(control + literal_bias);
            if (output >= output_end) {
                break;
            }
            continue;
        }

        packed = *source++;
        length = (cb_u16)(
                (packed >> 5) + RESOURCE_PAIR_LENGTH_BIAS);
        copy_source = output - (cb_u16)(
                ((((cb_u16)(
                        control & RESOURCE_PAIR_CONTROL_DISTANCE)) << 1)
                | ((packed >> 4) & 1u)) + 1u);
        do {
            *output++ = *copy_source++;
        } while (--length != 0u);
        if (output >= output_end) {
            break;
        }

        for (;;) {
            control = *source++;
            if ((cb_i8)control < 0) {
                break;
            }
            *output++ = control == 0u
                    ? 0u
                    : (cb_u8)(control + literal_bias);
            if (output >= output_end) {
                goto finished;
            }
        }

        length = (cb_u16)(
                ((packed >> 1) & 7u) + RESOURCE_PAIR_LENGTH_BIAS);
        copy_source = output - (cb_u16)(
                ((((cb_u16)(
                        control & RESOURCE_PAIR_CONTROL_DISTANCE)) << 1)
                | (packed & 1u)) + 1u);
        do {
            *output++ = *copy_source++;
        } while (--length != 0u);
        if (output >= output_end) {
            break;
        }
    }

finished:
    return source;
}
