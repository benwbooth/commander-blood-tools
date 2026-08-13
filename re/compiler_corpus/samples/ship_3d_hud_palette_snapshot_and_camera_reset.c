typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <string.h>
#define FAR far
#else
#define FAR
void FAR *_fmemcpy(void FAR *destination, const void FAR *source, u16 count);
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define GAME_DATA FAR
#else
#define GAME_DATA
#endif

#define HUD_PALETTE_FIRST 128u
#define HUD_PALETTE_BYTES (64u * 3u)

extern u8 GAME_DATA live_palette[768];
extern u8 GAME_DATA hud_palette_snapshot[HUD_PALETTE_BYTES];
extern i16 GAME_DATA camera_x;
extern i16 GAME_DATA camera_y;
extern i16 GAME_DATA camera_z;

void FAR draw_hud_element(void);
void FAR ship_3d_hud_palette_snapshot_and_camera_reset_probe(void);

#if defined(__WATCOMC__)
#pragma aux ship_3d_hud_palette_snapshot_and_camera_reset_probe \
        modify exact [bx dx]
#endif

void FAR ship_3d_hud_palette_snapshot_and_camera_reset_probe(void)
{
    draw_hud_element();
    _fmemcpy(
        (void FAR *)hud_palette_snapshot,
        (const void FAR *)(live_palette + HUD_PALETTE_FIRST * 3u),
        (u16)sizeof(hud_palette_snapshot));
    camera_x = 10000;
    camera_y = 12000;
    camera_z = 0;
}
