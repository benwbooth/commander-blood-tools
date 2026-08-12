/*
 * Codegen probe for BLOODPRG 0x009510.
 * This is not recovered game source.
 */
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 ui_state;
extern volatile i16 bridge_view_frame;

#if defined(__WATCOMC__)
#pragma aux presentation_mode_bits_update_probe value [ax] modify exact [ax]
#endif

u16 NEAR presentation_mode_bits_update_probe(void)
{
    u16 flags;
    u16 mode;
    i16 frame;

    flags = (u16)(ui_state & 0xff0fu);
    if ((flags & 2u) == 0) {
        mode = 1u;
        frame = bridge_view_frame;
        if (frame > 0x16 && frame <= 0x9d) {
            mode = (u16)(mode << 1);
            if (frame > 0x43) {
                mode = (u16)(mode << 1);
                if (frame > 0x70) {
                    mode = (u16)(mode << 1);
                }
            }
        }
        flags = (u16)(flags | (mode << 4));
    }

    ui_state = flags;
    return flags;
}
