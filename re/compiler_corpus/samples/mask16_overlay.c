/*
 * Codegen probe for BLOODPRG 0x007CB4.
 * This is not recovered game source.
 */
typedef signed char i8;
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define DISPLAY_AT(offset) \
    ((volatile u8 FAR *)MK_FP(FP_SEG(graphics_display_buffer), (offset)))
#else
#define FAR
#define NEAR
#define DISPLAY_AT(offset) (graphics_display_buffer + (offset))
#endif

extern volatile u8 FAR *graphics_display_buffer;
extern const u8 selected_mask_rows[][32];
extern volatile i8 selected_mask_index;

void NEAR mask16_overlay_probe(void)
{
    const u8 NEAR *source;
    volatile u8 FAR *row_pixels;
    unsigned row;

    source = selected_mask_rows[(int)selected_mask_index];
    row_pixels = DISPLAY_AT(0x12c5u);

    for (row = 0; row != 16u; ++row) {
        volatile u8 FAR *pixel;
        u16 bits;

        bits = (u16)((u16)source[0] << 8);
        bits = (u16)(bits | source[1]);
        source += 2;
        pixel = row_pixels;

        while (bits != 0) {
            if ((bits & 0x8000u) != 0) {
                *pixel = 0xfeu;
            }
            ++pixel;
            bits = (u16)(bits << 1);
        }

        row_pixels += 320u;
    }
}
