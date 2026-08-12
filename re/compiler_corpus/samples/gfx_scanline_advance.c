/* Codegen probe for BLOODPRG 0x00AD96. */

typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct gfx_scanline_state {
    u16 row_width;
    u16 row_offset;
    u8 rows_remaining;
    u8 row_count_high;
} gfx_scanline_state;

int NEAR gfx_scanline_advance_probe(gfx_scanline_state *state)
{
    --state->rows_remaining;
    if (state->rows_remaining == 0) {
        return 0;
    }

    state->row_offset = (u16)(state->row_offset + 320u);
    return 1;
}
