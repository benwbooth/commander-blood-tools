/* Codegen probe for BLOODPRG 0x00A914. */
typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

#define READ_BIT(value) \
    do { \
        if (control_mask == 0u) { \
            control_word = *(const volatile u16 FAR *)source; \
            source += 2; \
            control_mask = 0x8000u; \
        } \
        (value) = (u16)(control_word & control_mask); \
        control_mask >>= 1; \
    } while (0)

typedef struct resource_ad_header {
    u16 output_extent;
    u16 staging_extent;
    u8 flags;
    u8 checksum;
} resource_ad_header;

extern const volatile u8 FAR *NEAR resource_pair_lz_decode_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination,
        volatile u8 FAR *destination_end,
        u8 literal_bias);

void NEAR resource_payload_decode_ad_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination)
{
    const volatile resource_ad_header FAR *header;
    volatile u8 FAR *destination_end;
    volatile u8 FAR *staged_values;
    u16 control_mask;
    u16 control_word;
    u16 bit;
    u16 length;
    u16 pending_length;
    u16 index;
    u8 descriptor;
    u8 flags;
    u8 literal_bias;
    u8 value;

    header = (const volatile resource_ad_header FAR *)source;
    flags = header->flags;
    source += 6;
    if ((flags & 0x04u) == 0u) {
        for (index = 0u; index < 4u; ++index) {
            *destination++ = *source++;
        }
    }

    literal_bias = (flags & 0x40u) != 0u ? 0x80u : 0u;
    destination_end = destination + header->output_extent;
    staged_values = destination_end - header->staging_extent;
    source = resource_pair_lz_decode_probe(
            source, staged_values, destination_end, literal_bias);

    control_mask = 0u;
    pending_length = 0u;
    for (;;) {
        READ_BIT(bit);
        if (bit == 0u) {
            *destination++ = *staged_values++;
            continue;
        }

        value = *staged_values++;
        if ((flags & 0x80u) != 0u) {
            READ_BIT(bit);
            if (bit == 0u) {
                length = 0u;
            } else {
                READ_BIT(bit);
                if (bit == 0u) {
                    length = 2u;
                } else {
                    READ_BIT(bit);
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
            READ_BIT(bit);
            if (bit == 0u) {
                length = 2u;
            } else {
                READ_BIT(bit);
                if (bit == 0u) {
                    length = 3u;
                } else {
                    READ_BIT(bit);
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
            if (pending_length > 4u) {
                length = pending_length;
                pending_length = 0u;
            } else if (pending_length == 4u) {
                length = (u16)(*source++ + 20u);
                pending_length = 0u;
            } else {
                descriptor = *source++;
                length = (u16)(descriptor >> 4);
                if (length != 0u) {
                    length = (u16)(length + 4u);
                } else {
                    length = (u16)(*source++ + 20u);
                }
                pending_length = (u16)((descriptor & 0x0Fu) + 4u);
            }
        }

        do {
            *destination++ = value;
        } while (--length != 0u);
    }
}
