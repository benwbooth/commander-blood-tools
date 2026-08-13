/* Codegen probe for BLOODPRG 0x00A4ED. */
typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

void NEAR resource_rect_blit_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *framebuffer,
        u16 x,
        u16 y,
        u16 width,
        u16 row_mode)
{
    volatile u8 FAR *destination;
    u16 count;
    u16 pitch;
    u8 rows;
    u8 transparent;
    u8 value;

    destination = framebuffer + (u16)(
            ((y & 0x00FFu) << 8) +
            (y >> 8) +
            (y << 6) +
            x);
    rows = (u8)row_mode;
    transparent = (u8)(row_mode >> 8) == 0xFFu;

    if (width == 320u) {
        count = (u16)(width * rows);
        if (transparent != 0u) {
            do {
                value = *source++;
                if (value != 0u) {
                    *destination = value;
                }
                ++destination;
            } while (--count != 0u);
        } else {
            while (count != 0u) {
                *destination++ = *source++;
                --count;
            }
        }
        return;
    }

    pitch = (u16)(320u - width);
    do {
        count = width;
        if (transparent != 0u) {
            do {
                value = *source++;
                if (value != 0u) {
                    *destination = value;
                }
                ++destination;
            } while (--count != 0u);
        } else {
            while (count != 0u) {
                *destination++ = *source++;
                --count;
            }
        }
        destination += pitch;
    } while (--rows != 0u);
}
