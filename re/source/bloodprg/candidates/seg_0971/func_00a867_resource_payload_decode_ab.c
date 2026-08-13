#include "../include/bloodprg_list.h"

#define RESOURCE_DECODE_HEADER_BYTES 6u
#define RESOURCE_DECODE_CONTROL_SENTINEL 0x8000u
#define RESOURCE_DECODE_LONG_DISPLACEMENT_MASK 0xE000u

#define RESOURCE_DECODE_READ_BYTE(value) \
    do { \
        (value) = *source++; \
    } while (0)

#define RESOURCE_DECODE_READ_WORD(value) \
    do { \
        (value) = *(const volatile cb_u16 CB_FAR *)source; \
        source += 2; \
    } while (0)

#define RESOURCE_DECODE_READ_BIT(value) \
    do { \
        (value) = (cb_u16)(control_bits & 1u); \
        control_bits >>= 1; \
        if (control_bits == 0u) { \
            RESOURCE_DECODE_READ_WORD(control_word); \
            (value) = (cb_u16)(control_word & 1u); \
            control_bits = (cb_u16)( \
                    (control_word >> 1) | RESOURCE_DECODE_CONTROL_SENTINEL); \
        } \
    } while (0)

void CB_NEAR resource_payload_decode_ab(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination)
{
    const volatile cb_u8 CB_FAR *copy_source;
    cb_u16 control_bits;
    cb_u16 control_word;
    cb_u16 bit;
    cb_u16 length;
    cb_i16 displacement;
    cb_u8 value;

    resource_decode_mode = 1u;
    source += RESOURCE_DECODE_HEADER_BYTES;
    control_bits = 0u;

    for (;;) {
        RESOURCE_DECODE_READ_BIT(bit);
        if (bit != 0u) {
            RESOURCE_DECODE_READ_BYTE(value);
            *destination++ = value;
            continue;
        }

        RESOURCE_DECODE_READ_BIT(bit);
        if (bit == 0u) {
            length = 0u;
            RESOURCE_DECODE_READ_BIT(bit);
            length = (cb_u16)((length << 1) | bit);
            RESOURCE_DECODE_READ_BIT(bit);
            length = (cb_u16)((length << 1) | bit);
            RESOURCE_DECODE_READ_BYTE(value);
            displacement = (cb_i16)(cb_i8)value;
        } else {
            RESOURCE_DECODE_READ_WORD(control_word);
            length = (cb_u16)(control_word & 7u);
            displacement = (cb_i16)(
                    (control_word >> 3) |
                    RESOURCE_DECODE_LONG_DISPLACEMENT_MASK);
            if (length == 0u) {
                RESOURCE_DECODE_READ_BYTE(value);
                length = value;
                if (length == 0u) {
                    break;
                }
            }
        }

        length = (cb_u16)(length + 2u);
        copy_source = destination + displacement;
        do {
            *destination++ = *copy_source++;
        } while (--length != 0u);
    }
}
