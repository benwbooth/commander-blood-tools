#include "../include/bloodprg_graphics.h"

#define INTERPOLATE_RECT_FIELD(field) \
    do { \
        delta = (cb_i16)((cb_u16)source->field - (cb_u16)target->field); \
        quotient = (cb_i8)(delta / total_steps); \
        interpolated.field = (cb_i16)( \
                (cb_u16)target->field + \
                (cb_u16)((cb_i16)quotient * \
                    (cb_i16)(cb_i8)framebuffer_transition_current_step)); \
    } while (0)

void CB_FAR framebuffer_rect_interpolate_and_remap_step(
        const bloodprg_rect_i16 CB_NEAR *source,
        const bloodprg_rect_i16 CB_NEAR *target)
{
    bloodprg_rect_i16 interpolated;
    cb_i16 delta;
    cb_i8 quotient;
    cb_i8 total_steps;

#if defined(__WATCOMC__)
    _asm push ax;
#endif

    total_steps = (cb_i8)framebuffer_transition_total_steps;
    if ((cb_u8)total_steps == framebuffer_transition_current_step) {
#if defined(__WATCOMC__)
        _asm pop ax;
        _asm stc;
#endif
        return;
    }

    ++framebuffer_transition_current_step;
    INTERPOLATE_RECT_FIELD(x);
    INTERPOLATE_RECT_FIELD(y);
    INTERPOLATE_RECT_FIELD(width);
    INTERPOLATE_RECT_FIELD(height);

#if defined(__WATCOMC__)
    framebuffer_rect_palette_remap_ds_bp(
        framebuffer_transition_remap_table,
        (cb_u16)interpolated.x,
        (cb_u16)interpolated.y,
        (cb_u16)interpolated.width,
        (cb_u16)interpolated.height);
    _asm pop ax;
    _asm clc;
#else
    framebuffer_rect_palette_remap(
        framebuffer_transition_remap_table,
        (cb_u16)interpolated.x,
        (cb_u16)interpolated.y,
        (cb_u16)interpolated.width,
        (cb_u16)interpolated.height);
#endif
}

#undef INTERPOLATE_RECT_FIELD
