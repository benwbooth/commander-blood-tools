/* Codegen probe for BLOODPRG 0x00AB25. */
typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))

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

typedef struct resource_rect_header {
    u16 staging_extent;
    u16 staging_input_extent;
    u8 flags;
    u8 checksum;
} resource_rect_header;

typedef struct scanline_state {
    u16 row_width;
    u16 row_offset;
    u8 rows_remaining;
    u8 row_count_high;
} scanline_state;

extern volatile u16 GAME_DATA decode_mode;
extern const volatile u8 FAR *NEAR resource_pair_lz_decode_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination,
        volatile u8 FAR *destination_end,
        u8 literal_bias);
extern int NEAR gfx_scanline_advance_probe(scanline_state *state);

void NEAR resource_payload_decode_rect_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *staging,
        volatile u8 FAR *framebuffer,
        u16 vertical_offset,
        u16 row_width,
        u16 rows)
{
    const volatile resource_rect_header FAR *header;
    volatile u8 FAR *framebuffer_base;
    volatile u8 FAR *staging_end;
    volatile u8 FAR *staged_values;
    scanline_state scanline;
    u16 control_mask;
    u16 control_word;
    u16 bit;
    u16 length;
    u16 chunk;
    u16 pending_length;
    u16 x;
    u16 y;
    u8 descriptor;
    u8 flags;
    u8 literal_bias;
    u8 value;

    decode_mode = 3u;
    header = (const volatile resource_rect_header FAR *)source;
    flags = header->flags;
    source += 6;
    x = 0u;
    y = 0u;
    if ((flags & 0x04u) == 0u) {
        x = *(const volatile u16 FAR *)source;
        y = *(const volatile u16 FAR *)(source + 2);
        source += 4;
    }

    literal_bias = (flags & 0x40u) != 0u ? 0x80u : 0u;
    staging_end = staging + header->staging_extent;
    staged_values = staging_end - header->staging_input_extent;
    source = resource_pair_lz_decode_probe(
            source, staged_values, staging_end, literal_bias);

    y = (u16)(y + vertical_offset);
    scanline.row_width = (u16)(row_width & 0x01FFu);
    scanline.row_offset = (u16)(
            ((y & 0x00FFu) << 8) + (y >> 8) + (y << 6) + x);
    scanline.rows_remaining = (u8)rows;
    scanline.row_count_high = 0u;
    if (scanline.rows_remaining > 0x82u) {
        scanline.rows_remaining = 0x82u;
    }

    control_mask = 0u;
    pending_length = 0u;
    row_width = scanline.row_width;
    framebuffer_base = framebuffer;
    framebuffer += scanline.row_offset;
    for (;;) {
        READ_BIT(bit);
        if (bit == 0u) {
            value = *staged_values++;
            length = 1u;
        } else {
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
                        length = bit == 0u ? 3u : 4u;
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
                        length = bit == 0u ? 4u : 0u;
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
        }

        while (length != 0u) {
            chunk = length < row_width ? length : row_width;
            length = (u16)(length - chunk);
            row_width = (u16)(row_width - chunk);
            if (value != 0u) {
                do {
                    *framebuffer++ = value;
                } while (--chunk != 0u);
            } else {
                framebuffer += chunk;
            }

            if (row_width != 0u) {
                continue;
            }
            if (!gfx_scanline_advance_probe(&scanline)) {
                return;
            }
            framebuffer = framebuffer_base + scanline.row_offset;
            row_width = scanline.row_width;
        }
    }
}
