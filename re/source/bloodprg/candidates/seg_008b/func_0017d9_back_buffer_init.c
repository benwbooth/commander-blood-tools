#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

cb_i16 CB_FAR back_buffer_init(void)
{
    bloodprg_graphics_buffer_ptr saved_framebuffer;
    cb_i16 load_result;

    pbm_palette_refresh = 0u;
    pbm_transparent_zero = 0u;
    load_result = pbm_image_load_and_decode(
            back_buffer_init_image_path,
            graphics_back_buffer);

    saved_framebuffer = graphics_draw_framebuffer;
    graphics_draw_framebuffer =
            (bloodprg_graphics_buffer_ptr)MK_FP(0xA000u, 0xC000u);
    chunky_to_planar_framebuffer(graphics_back_buffer);
    graphics_draw_framebuffer = saved_framebuffer;
    return load_result;
}
