/*
 * Codegen probe for BLOODPRG 0x00A117. The aggregate assignment tests whether
 * a compiler recognizes the fixed 384-byte copy as an inline string move. The
 * Watcom-only pragma states observed clobbers; it supplies no instruction code.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned long u32;

typedef struct palette_low_block {
    u32 dwords[0x60];
} palette_low_block;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 render_update_flag_2751;
extern palette_low_block palette_low_5251;
extern palette_low_block palette_low_5851;

void NEAR flag_gated_2751_probe(void);

#if defined(__WATCOMC__)
#pragma aux flag_gated_2751_probe modify exact [cx di]
#endif

void NEAR flag_gated_2751_probe(void)
{
    if (render_update_flag_2751 & 1u) {
        return;
    }

    palette_low_5851 = palette_low_5251;
}
