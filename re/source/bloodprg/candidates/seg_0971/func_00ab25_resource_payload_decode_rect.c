#include <string.h>

#include "../include/bloodprg_list.h"
#include "../include/bloodprg_graphics.h"

#define RESOURCE_RECT_HEADER_BYTES 6u
#define RESOURCE_RECT_COORDINATE_BYTES 4u
#define RESOURCE_RECT_FLAG_NO_COORDINATES 0x04u
#define RESOURCE_RECT_FLAG_HIGH_LITERAL_BIAS 0x40u
#define RESOURCE_RECT_FLAG_HIGH_TOKEN_LAYOUT 0x80u
#define RESOURCE_RECT_LITERAL_BIAS_HIGH 0x80u
#define RESOURCE_RECT_WIDTH_MASK 0x01FFu
#define RESOURCE_RECT_MAX_ROWS 0x82u
#define RESOURCE_RECT_EXTENDED_LENGTH_BIAS 20u
#define RESOURCE_RECT_NIBBLE_LENGTH_BIAS 4u

#define RESOURCE_RECT_READ_BIT(value) \
    do { \
        if (control_mask == 0u) { \
            control_word = *(const volatile cb_u16 CB_FAR *)source; \
            source += 2; \
            control_mask = 0x8000u; \
        } \
        (value) = (cb_u16)(control_word & control_mask); \
        control_mask >>= 1; \
    } while (0)

typedef struct resource_rect_header {
    cb_u16 staging_extent;
    cb_u16 staging_input_extent;
    cb_u8 flags;
    cb_u8 checksum;
} resource_rect_header;

typedef char resource_rect_header_size_must_be_6[
        sizeof(resource_rect_header) == RESOURCE_RECT_HEADER_BYTES ? 1 : -1];

void CB_NEAR resource_payload_decode_rect(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *staging,
        volatile cb_u8 CB_FAR *framebuffer,
        cb_u16 vertical_offset,
        cb_u16 row_width,
        cb_u16 rows)
{
    const volatile resource_rect_header CB_FAR *header;
    volatile cb_u8 CB_FAR *framebuffer_base;
    volatile cb_u8 CB_FAR *staging_end;
    volatile cb_u8 CB_FAR *staged_values;
    bloodprg_gfx_scanline_state scanline;
    cb_u16 control_mask;
    cb_u16 control_word;
    cb_u16 bit;
    cb_u16 length;
    cb_u16 chunk;
    cb_u16 pending_length;
    cb_u16 x;
    cb_u16 y;
    cb_u8 descriptor;
    cb_u8 flags;
    cb_u8 literal_bias;
    cb_u8 value;

    resource_decode_mode = 3u;
    header = (const volatile resource_rect_header CB_FAR *)source;
    flags = header->flags;
    source += RESOURCE_RECT_HEADER_BYTES;
    x = 0u;
    y = 0u;
    if ((flags & RESOURCE_RECT_FLAG_NO_COORDINATES) == 0u) {
        x = *(const volatile cb_u16 CB_FAR *)source;
        y = *(const volatile cb_u16 CB_FAR *)(source + 2);
        source += RESOURCE_RECT_COORDINATE_BYTES;
    }

    literal_bias = (flags & RESOURCE_RECT_FLAG_HIGH_LITERAL_BIAS) != 0u
            ? RESOURCE_RECT_LITERAL_BIAS_HIGH
            : 0u;
    staging_end = staging + header->staging_extent;
    staged_values = staging_end - header->staging_input_extent;
    source = resource_pair_lz_decode(
            source,
            staged_values,
            staging_end,
            literal_bias);

    y = (cb_u16)(y + vertical_offset);
    scanline.row_width = (cb_u16)(row_width & RESOURCE_RECT_WIDTH_MASK);
    scanline.row_offset = (cb_u16)(
            ((y & 0x00FFu) << 8) +
            (y >> 8) +
            (y << 6) +
            x);
    scanline.rows_remaining = (cb_u8)rows;
    scanline.row_count_high = 0u;
    if (scanline.rows_remaining > RESOURCE_RECT_MAX_ROWS) {
        scanline.rows_remaining = RESOURCE_RECT_MAX_ROWS;
    }

    control_mask = 0u;
    pending_length = 0u;
    row_width = scanline.row_width;
    framebuffer_base = framebuffer;
    framebuffer += scanline.row_offset;
    for (;;) {
        RESOURCE_RECT_READ_BIT(bit);
        if (bit == 0u) {
            value = *staged_values++;
            length = 1u;
        } else {
            value = *staged_values++;
            if ((flags & RESOURCE_RECT_FLAG_HIGH_TOKEN_LAYOUT) != 0u) {
                RESOURCE_RECT_READ_BIT(bit);
                if (bit == 0u) {
                    length = 0u;
                } else {
                    RESOURCE_RECT_READ_BIT(bit);
                    if (bit == 0u) {
                        length = 2u;
                    } else {
                        RESOURCE_RECT_READ_BIT(bit);
                        length = bit == 0u ? 3u : 4u;
                    }
                }
            } else {
                RESOURCE_RECT_READ_BIT(bit);
                if (bit == 0u) {
                    length = 2u;
                } else {
                    RESOURCE_RECT_READ_BIT(bit);
                    if (bit == 0u) {
                        length = 3u;
                    } else {
                        RESOURCE_RECT_READ_BIT(bit);
                        length = bit == 0u ? 4u : 0u;
                    }
                }
            }

            if (length == 0u) {
                if (pending_length > RESOURCE_RECT_NIBBLE_LENGTH_BIAS) {
                    length = pending_length;
                    pending_length = 0u;
                } else if (pending_length == RESOURCE_RECT_NIBBLE_LENGTH_BIAS) {
                    length = (cb_u16)(
                            *source++ + RESOURCE_RECT_EXTENDED_LENGTH_BIAS);
                    pending_length = 0u;
                } else {
                    descriptor = *source++;
                    length = (cb_u16)(descriptor >> 4);
                    if (length != 0u) {
                        length = (cb_u16)(
                                length + RESOURCE_RECT_NIBBLE_LENGTH_BIAS);
                    } else {
                        length = (cb_u16)(
                                *source++ + RESOURCE_RECT_EXTENDED_LENGTH_BIAS);
                    }
                    pending_length = (cb_u16)(
                            (descriptor & 0x0Fu) +
                            RESOURCE_RECT_NIBBLE_LENGTH_BIAS);
                }
            }
        }

        while (length != 0u) {
            chunk = length < row_width ? length : row_width;
            length = (cb_u16)(length - chunk);
            row_width = (cb_u16)(row_width - chunk);
            if (value != 0u) {
#if defined(__WATCOMC__)
                cb_u16 repeated_word;
                cb_u32 repeated_dword;

                repeated_word = (cb_u16)(value | ((cb_u16)value << 8));
                repeated_dword = (cb_u32)repeated_word
                        | ((cb_u32)repeated_word << 16);
                switch (chunk) {
                case 1u:
                    *framebuffer = value;
                    break;
                case 2u:
                    *(volatile cb_u16 CB_FAR *)framebuffer = repeated_word;
                    break;
                case 3u:
                    *(volatile cb_u16 CB_FAR *)framebuffer = repeated_word;
                    framebuffer[2] = value;
                    break;
                case 4u:
                    *(volatile cb_u32 CB_FAR *)framebuffer = repeated_dword;
                    break;
                default:
                    _fmemset((void CB_FAR *)framebuffer, value, chunk);
                    break;
                }
                framebuffer += chunk;
#else
                do {
                    *framebuffer++ = value;
                } while (--chunk != 0u);
#endif
            } else {
                framebuffer += chunk;
            }

            if (row_width != 0u) {
                continue;
            }
            if (!gfx_scanline_advance(&scanline)) {
                return;
            }
            framebuffer = framebuffer_base + scanline.row_offset;
            row_width = scanline.row_width;
        }
    }
}
