#include "../include/bloodprg_list.h"

#define RESOURCE_AD_HEADER_BYTES 6u
#define RESOURCE_AD_PREFIX_BYTES 4u
#define RESOURCE_AD_FLAG_NO_PREFIX 0x04u
#define RESOURCE_AD_FLAG_HIGH_LITERAL_BIAS 0x40u
#define RESOURCE_AD_FLAG_HIGH_TOKEN_LAYOUT 0x80u
#define RESOURCE_AD_LITERAL_BIAS_HIGH 0x80u
#define RESOURCE_AD_EXTENDED_LENGTH_BIAS 20u
#define RESOURCE_AD_NIBBLE_LENGTH_BIAS 4u

#define RESOURCE_AD_READ_BIT(value) \
    do { \
        if (control_mask == 0u) { \
            control_word = *(const volatile cb_u16 CB_FAR *)source; \
            source += 2; \
            control_mask = 0x8000u; \
        } \
        (value) = (cb_u16)(control_word & control_mask); \
        control_mask >>= 1; \
    } while (0)

typedef struct resource_ad_header {
    cb_u16 output_extent;
    cb_u16 staging_extent;
    cb_u8 flags;
    cb_u8 checksum;
} resource_ad_header;

typedef char resource_ad_header_size_must_be_6[
        sizeof(resource_ad_header) == RESOURCE_AD_HEADER_BYTES ? 1 : -1];

void CB_NEAR resource_payload_decode_ad(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination)
{
    const volatile resource_ad_header CB_FAR *header;
    volatile cb_u8 CB_FAR *destination_end;
    volatile cb_u8 CB_FAR *staged_values;
    cb_u16 control_mask;
    cb_u16 control_word;
    cb_u16 bit;
    cb_u16 length;
    cb_u16 pending_length;
    cb_u16 index;
    cb_u8 descriptor;
    cb_u8 flags;
    cb_u8 literal_bias;
    cb_u8 value;

    header = (const volatile resource_ad_header CB_FAR *)source;
    flags = header->flags;
    source += RESOURCE_AD_HEADER_BYTES;
    if ((flags & RESOURCE_AD_FLAG_NO_PREFIX) == 0u) {
        for (index = 0u; index < RESOURCE_AD_PREFIX_BYTES; ++index) {
            *destination++ = *source++;
        }
    }

    literal_bias = (flags & RESOURCE_AD_FLAG_HIGH_LITERAL_BIAS) != 0u
            ? RESOURCE_AD_LITERAL_BIAS_HIGH
            : 0u;
    destination_end = destination + header->output_extent;
    staged_values = destination_end - header->staging_extent;
    source = resource_pair_lz_decode(
            source,
            staged_values,
            destination_end,
            literal_bias);

    control_mask = 0u;
    pending_length = 0u;
    for (;;) {
        RESOURCE_AD_READ_BIT(bit);
        if (bit == 0u) {
            *destination++ = *staged_values++;
            continue;
        }

        value = *staged_values++;
        if ((flags & RESOURCE_AD_FLAG_HIGH_TOKEN_LAYOUT) != 0u) {
            RESOURCE_AD_READ_BIT(bit);
            if (bit == 0u) {
                length = 0u;
            } else {
                RESOURCE_AD_READ_BIT(bit);
                if (bit == 0u) {
                    length = 2u;
                } else {
                    RESOURCE_AD_READ_BIT(bit);
                    if (bit == 0u) {
                        length = 3u;
                    } else {
                        if (destination >= destination_end) {
                            break;
                        }
                        length = 4u;
                    }
                }
            }
        } else {
            RESOURCE_AD_READ_BIT(bit);
            if (bit == 0u) {
                length = 2u;
            } else {
                RESOURCE_AD_READ_BIT(bit);
                if (bit == 0u) {
                    length = 3u;
                } else {
                    RESOURCE_AD_READ_BIT(bit);
                    if (bit == 0u) {
                        length = 4u;
                    } else {
                        if (destination >= destination_end) {
                            break;
                        }
                        length = 0u;
                    }
                }
            }
        }

        if (length == 0u) {
            if (pending_length > RESOURCE_AD_NIBBLE_LENGTH_BIAS) {
                length = pending_length;
                pending_length = 0u;
            } else if (pending_length == RESOURCE_AD_NIBBLE_LENGTH_BIAS) {
                length = (cb_u16)(*source++ + RESOURCE_AD_EXTENDED_LENGTH_BIAS);
                pending_length = 0u;
            } else {
                descriptor = *source++;
                length = (cb_u16)(descriptor >> 4);
                if (length != 0u) {
                    length = (cb_u16)(length + RESOURCE_AD_NIBBLE_LENGTH_BIAS);
                } else {
                    length = (cb_u16)(
                            *source++ + RESOURCE_AD_EXTENDED_LENGTH_BIAS);
                }
                pending_length = (cb_u16)(
                        (descriptor & 0x0Fu) + RESOURCE_AD_NIBBLE_LENGTH_BIAS);
            }
        }

        do {
            *destination++ = value;
        } while (--length != 0u);
    }
}
