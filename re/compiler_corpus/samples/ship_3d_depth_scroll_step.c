/*
 * Codegen probe for BLOODPRG 0x00B75C.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef union depth_word {
    u16 value;
    struct {
        u8 low;
        u8 high;
    } byte;
} depth_word;

extern volatile u16 depth_offset;
extern volatile u8 depth_opening;
extern volatile u8 depth_closing;
extern volatile u8 depth_step;

#if defined(__WATCOMC__)
#pragma aux ship_3d_depth_scroll_step_probe modify exact [ax]
#endif

void NEAR ship_3d_depth_scroll_step_probe(void)
{
    depth_word depth;

    if ((depth_opening & 1u) != 0) {
        depth.value = depth_offset;
        if (depth.value == 0x0041u) {
            depth_opening = 0;
            return;
        }

        depth.byte.low += depth_step;
        if ((i16)depth.value < (i16)0x0041) {
            depth_offset = depth.value;
        } else {
            depth_offset = 0x0041u;
        }
        return;
    }

    if ((depth_closing & 1u) == 0) {
        return;
    }

    depth.value = depth_offset;
    if (depth.value == 0) {
        depth_closing = 0;
        return;
    }

    depth.byte.low -= depth_step;
    if ((i8)depth.byte.low >= 0) {
        depth_offset = depth.value;
    } else {
        depth_offset = 0;
    }
}
