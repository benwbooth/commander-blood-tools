typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 palette_dirty;
extern volatile u8 live_palette[768];
extern volatile u8 mouse_primary_pressed;
extern volatile u8 mouse_press_pending;

void FAR video_retrace_phase_wait(void);
void FAR vga_palette_write(const volatile u8 *palette);
void NEAR palette_upload_if_dirty_probe(void);

#if defined(__WATCOMC__)
#pragma aux video_retrace_phase_wait modify exact [si]
#pragma aux vga_palette_write parm [si]
#pragma aux palette_upload_if_dirty_probe modify exact [ax bx cx dx si di es]
#endif

void NEAR palette_upload_if_dirty_probe(void)
{
#if defined(__WATCOMC__)
    _asm push ax;
#endif

    if ((palette_dirty & 1u) != 0) {
        video_retrace_phase_wait();
        vga_palette_write(live_palette);
        palette_dirty = 0;
        mouse_press_pending = 0;
        mouse_primary_pressed = 0;
    }

#if defined(__WATCOMC__)
    _asm pop ax;
#endif
}
