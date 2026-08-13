/* Codegen probe for BLOODPRG 0x001E5D. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct rect_i16 {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16;

extern const u8 NEAR *transition_remap_table;
extern volatile u8 transition_total_steps;
extern volatile u8 transition_current_step;

void FAR framebuffer_rect_palette_remap_probe(
        const u8 FAR *remap_table,
        u16 x,
        u16 y,
        u16 width,
        u16 height);
void FAR framebuffer_rect_palette_remap_ds_bp_probe(
        const u8 NEAR *remap_table,
        u16 x,
        u16 y,
        u16 width,
        u16 height);
void FAR framebuffer_rect_interpolate_and_remap_step_probe(
        const rect_i16 NEAR *source,
        const rect_i16 NEAR *target);

#if defined(__WATCOMC__)
#pragma aux framebuffer_rect_palette_remap_ds_bp_probe = \
        "push bp" \
        "mov bp,ax" \
        "call far ptr framebuffer_rect_palette_remap_probe" \
        "pop bp" \
        parm [si] [bx] [cx] [dx] [ax] modify exact []
#pragma aux framebuffer_rect_interpolate_and_remap_step_probe \
        parm [si] [di] modify exact []
#endif

#define INTERPOLATE_RECT_FIELD(field) \
    do { \
        delta = (i16)((u16)source->field - (u16)target->field); \
        quotient = (i8)(delta / total_steps); \
        interpolated.field = (i16)( \
                (u16)target->field + \
                (u16)((i16)quotient * \
                    (i16)(i8)transition_current_step)); \
    } while (0)

void FAR framebuffer_rect_interpolate_and_remap_step_probe(
        const rect_i16 NEAR *source,
        const rect_i16 NEAR *target)
{
    rect_i16 interpolated;
    i16 delta;
    i8 quotient;
    i8 total_steps;

#if defined(__WATCOMC__)
    _asm push ax;
#endif

    total_steps = (i8)transition_total_steps;
    if ((u8)total_steps == transition_current_step) {
#if defined(__WATCOMC__)
        _asm pop ax;
        _asm stc;
#endif
        return;
    }

    ++transition_current_step;
    INTERPOLATE_RECT_FIELD(x);
    INTERPOLATE_RECT_FIELD(y);
    INTERPOLATE_RECT_FIELD(width);
    INTERPOLATE_RECT_FIELD(height);

#if defined(__WATCOMC__)
    framebuffer_rect_palette_remap_ds_bp_probe(
        transition_remap_table,
        (u16)interpolated.x,
        (u16)interpolated.y,
        (u16)interpolated.width,
        (u16)interpolated.height);
    _asm pop ax;
    _asm clc;
#else
    framebuffer_rect_palette_remap_probe(
        transition_remap_table,
        (u16)interpolated.x,
        (u16)interpolated.y,
        (u16)interpolated.width,
        (u16)interpolated.height);
#endif
}

#undef INTERPOLATE_RECT_FIELD
