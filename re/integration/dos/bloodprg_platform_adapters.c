#include <dos.h>

#include "../../source/bloodprg/candidates/include/bloodprg_ems.h"
#include "../../source/bloodprg/candidates/include/bloodprg_graphics.h"
#include "../../source/bloodprg/candidates/include/bloodprg_resource.h"

static int bloodprg_dos_call_far_path(
        union REGS *registers,
        struct SREGS *segments,
        const volatile char CB_FAR *path)
{
    segread(segments);
    segments->ds = FP_SEG(path);
    registers->x.dx = FP_OFF(path);
    int86x(0x21, registers, registers, segments);
    return registers->x.cflag == 0;
}

volatile bloodprg_dos_dta CB_FAR *CB_NEAR cb_dos_get_dta(void)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x2f00u;
    segread(&segments);
    int86x(0x21, &registers, &registers, &segments);
    return (volatile bloodprg_dos_dta CB_FAR *)MK_FP(
            segments.es, registers.x.bx);
}

int CB_NEAR cb_dos_find_first(const volatile char CB_FAR *path)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x4e00u;
    registers.x.cx = 0u;
    return bloodprg_dos_call_far_path(&registers, &segments, path);
}

int CB_NEAR cb_dos_open_read_only(
        const volatile char CB_FAR *path, cb_u16 *handle)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x3d00u;
    if (bloodprg_dos_call_far_path(&registers, &segments, path)) {
        *handle = registers.x.ax;
        return 1;
    }
    *handle = registers.x.ax;
    return 0;
}

int CB_NEAR cb_dos_create_truncate(
        const volatile char CB_FAR *path, cb_u16 *handle)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x3c00u;
    registers.x.cx = 0u;
    if (bloodprg_dos_call_far_path(&registers, &segments, path)) {
        *handle = registers.x.ax;
        return 1;
    }
    *handle = registers.x.ax;
    return 0;
}

int CB_NEAR cb_dos_delete(const volatile char CB_FAR *path)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x4100u;
    return bloodprg_dos_call_far_path(&registers, &segments, path);
}

void CB_NEAR cb_dos_seek_absolute(cb_u16 handle, cb_u32 offset)
{
    union REGS registers;

    registers.x.ax = 0x4200u;
    registers.x.bx = handle;
    registers.x.cx = (cb_u16)(offset >> 16);
    registers.x.dx = (cb_u16)offset;
    int86(0x21, &registers, &registers);
}

cb_u16 CB_NEAR cb_dos_read(
        cb_u16 handle,
        volatile cb_u8 CB_FAR *destination,
        cb_u16 byte_count)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x3f00u;
    registers.x.bx = handle;
    registers.x.cx = byte_count;
    segread(&segments);
    segments.ds = FP_SEG(destination);
    registers.x.dx = FP_OFF(destination);
    int86x(0x21, &registers, &registers, &segments);
    return registers.x.ax;
}

void CB_NEAR cb_dos_close(cb_u16 handle)
{
    union REGS registers;

    registers.x.ax = 0x3e00u;
    registers.x.bx = handle;
    int86(0x21, &registers, &registers);
}

int CB_NEAR cb_dos_create_game_file(
        const volatile char CB_GAME_DATA *path,
        volatile cb_u16 CB_GAME_DATA *handle)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x3c00u;
    registers.x.cx = 0u;
    if (bloodprg_dos_call_far_path(
            &registers, &segments, (const volatile char CB_FAR *)path)) {
        *handle = registers.x.ax;
        return 1;
    }
    *handle = registers.x.ax;
    return 0;
}

cb_u16 CB_NEAR cb_dos_write(
        cb_u16 handle,
        const volatile cb_u8 CB_FAR *source,
        cb_u16 byte_count)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x4000u;
    registers.x.bx = handle;
    registers.x.cx = byte_count;
    segread(&segments);
    segments.ds = FP_SEG(source);
    registers.x.dx = FP_OFF(source);
    int86x(0x21, &registers, &registers, &segments);
    return registers.x.ax;
}

void CB_NEAR cb_ems_map_page(
        cb_u16 handle, cb_u16 logical_page, cb_u8 physical_page)
{
    union REGS registers;

    registers.x.ax = (cb_u16)(0x4400u | physical_page);
    registers.x.bx = handle;
    registers.x.dx = logical_page;
    int86(0x67, &registers, &registers);
}

void CB_FAR cb_resource_allocation_failure(cb_u16 error_code)
{
    error_overlay_draw(error_code, (const cb_u8 CB_FAR *)0);
}

/* These names are source-integration bridges for callers that have a far
 * pointer while the recovered routine's proven body consumes DS-relative
 * text or source. The original callers establish that segment before entry. */
cb_i16 CB_FAR pbm_image_load_and_decode_c(
        volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *file_buffer_end)
{
    return pbm_image_load_and_decode(path, file_buffer_end);
}

cb_u16 CB_FAR text_width_dual_font_far(
        const cb_u8 CB_FAR *text, int use_main_font)
{
    return text_width_dual_font(
            (const cb_u8 CB_NEAR *)text, use_main_font);
}

void CB_FAR fullscreen_copy_to_backbuffer_far(
        const cb_u32 CB_FAR *source)
{
    fullscreen_copy_to_backbuffer((const cb_u32 CB_NEAR *)source);
}

void CB_FAR small_text_render_far(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color)
{
    small_text_render((const cb_u8 CB_NEAR *)text, x, y, color);
}
