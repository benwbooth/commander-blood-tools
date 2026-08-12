/*
 * Codegen probe for BLOODPRG 0x009B04.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define GAME_DATA far
#else
#define GAME_DATA
#endif

typedef struct projection_context {
    i32 matrix[9];
    u16 projected_x;
    u16 projected_y;
    u16 projected_depth;
} projection_context;

extern volatile i16 GAME_DATA clip_left;
extern volatile i16 GAME_DATA clip_right;
extern volatile i16 GAME_DATA clip_top;
extern volatile i16 GAME_DATA clip_bottom;

void NEAR ship_3d_plot_point_probe(
        const volatile projection_context GAME_DATA *projection,
        volatile u8 FAR *framebuffer)
{
    i16 x;
    i16 y;
    u16 offset;
    u8 shade;

    x = (i16)projection->projected_x;
    if (x < clip_left || x >= clip_right) {
        return;
    }

    y = (i16)projection->projected_y;
    if (y < clip_top || y >= clip_bottom) {
        return;
    }

    offset = (u16)((u16)y * 320u + (u16)x);
    if (framebuffer[offset] != 0) {
        return;
    }

    shade = (u8)(0xefu - (projection->projected_depth >> 12));
    framebuffer[offset] = shade;
}
