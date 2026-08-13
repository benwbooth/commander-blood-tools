#include <dos.h>

typedef unsigned char u8;
typedef signed int i16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_back_buffer;
extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_draw_framebuffer;
extern volatile u8 __based(__segname("GAME_DATA")) pbm_palette_refresh;
extern volatile u8 __based(__segname("GAME_DATA")) pbm_transparent_zero;
extern volatile char __based(__segname("GAME_DATA"))
        back_buffer_init_image_path[];

i16 far pbm_image_load_and_decode(
        volatile char far *path,
        volatile u8 far *file_buffer_end);
void far chunky_to_planar_framebuffer(const volatile u8 far *source);
i16 far back_buffer_init(void);

#pragma aux pbm_image_load_and_decode \
        parm [ds si] [es di] value [ax] modify exact [ax]
#pragma aux chunky_to_planar_framebuffer parm [ds si] modify exact [dx]
#pragma aux back_buffer_init value [ax] modify exact [ax dx]

i16 far back_buffer_init(void)
{
    graphics_buffer_ptr saved_framebuffer;
    i16 load_result;

    pbm_palette_refresh = 0u;
    pbm_transparent_zero = 0u;
    load_result = pbm_image_load_and_decode(
            back_buffer_init_image_path,
            graphics_back_buffer);

    saved_framebuffer = graphics_draw_framebuffer;
    graphics_draw_framebuffer =
            (graphics_buffer_ptr)MK_FP(0xA000u, 0xC000u);
    chunky_to_planar_framebuffer(graphics_back_buffer);
    graphics_draw_framebuffer = saved_framebuffer;
    return load_result;
}
