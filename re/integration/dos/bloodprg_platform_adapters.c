#include <dos.h>

#include "../../source/bloodprg/candidates/include/bloodprg_audio.h"
#include "../../source/bloodprg/candidates/include/bloodprg_ems.h"
#include "../../source/bloodprg/candidates/include/bloodprg_graphics.h"
#include "../../source/bloodprg/candidates/include/bloodprg_manu3.h"
#include "../../source/bloodprg/candidates/include/bloodprg_resource.h"

#if defined(BLOODPRG_ADAPTER_TRACE)
#pragma pack(1)
typedef struct bloodprg_adapter_trace_record {
    char magic[8];
    cb_u16 open_call_count;
    cb_u16 path_offset;
    cb_u16 path_segment;
    cb_u16 handle_before;
    cb_u16 dos_ax;
    cb_u16 carry;
    cb_u16 success;
    cb_u16 handle_after;
    char path[16];
} bloodprg_adapter_trace_record;
#pragma pack()

volatile bloodprg_adapter_trace_record bloodprg_adapter_trace = {
    "CBOPEN1", 0, 0, 0, 0, 0, 0, 0, 0, ""
};
#endif

#if defined(__WATCOMC__)
static int bloodprg_dos_find_first_interrupt(
        const volatile char CB_FAR *path);
#pragma aux bloodprg_dos_find_first_interrupt = \
        "push ds" \
        "mov ds,dx" \
        "mov dx,ax" \
        "xor cx,cx" \
        "mov ax,4e00h" \
        "int 21h" \
        "sbb ax,ax" \
        "inc ax" \
        "pop ds" \
        parm [dx ax] value [ax] modify exact [ax cx dx]
#endif

static volatile bloodprg_dos_dta bloodprg_dos_dta_buffer;

#if defined(__WATCOMC__)
static void bloodprg_overlay_call_inherited_bp(
        bloodprg_overlay_entry_raw entry,
        const volatile void CB_NEAR *request);
#pragma aux bloodprg_overlay_call_inherited_bp = \
        "push bp" \
        "push ds" \
        "push es" \
        "push fs" \
        "mov bp,si" \
        "push dx" \
        "push ax" \
        "mov bx,sp" \
        "call dword ptr ss:[bx]" \
        "add sp,4" \
        "pop fs" \
        "pop es" \
        "pop ds" \
        "pop bp" \
        parm [dx ax] [si] modify exact [ax bx cx dx si di]
#endif

void CB_NEAR cb_overlay_call_inherited_bp(
        bloodprg_overlay_entry_raw entry,
        const volatile void CB_NEAR *request)
{
    bloodprg_overlay_call_inherited_bp(entry, request);
}

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
    volatile bloodprg_dos_dta CB_FAR *dta;

    dta = (volatile bloodprg_dos_dta CB_FAR *)&bloodprg_dos_dta_buffer;
    registers.x.ax = 0x1a00u;
    segread(&segments);
    segments.ds = FP_SEG(dta);
    registers.x.dx = FP_OFF(dta);
    int86x(0x21, &registers, &registers, &segments);
    return dta;
}

int CB_NEAR cb_dos_find_first(const volatile char CB_FAR *path)
{
#if defined(__WATCOMC__)
    return bloodprg_dos_find_first_interrupt(path);
#else
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x4e00u;
    registers.x.cx = 0u;
    return bloodprg_dos_call_far_path(&registers, &segments, path);
#endif
}

int CB_NEAR cb_dos_open_read_only(
        const volatile char CB_FAR *path, cb_u16 *handle)
{
    union REGS registers;
    struct SREGS segments;
#if defined(BLOODPRG_ADAPTER_TRACE)
    cb_u16 character_index;
#endif
    int success;

    registers.x.ax = 0x3d00u;
#if defined(BLOODPRG_ADAPTER_TRACE)
    ++bloodprg_adapter_trace.open_call_count;
    bloodprg_adapter_trace.path_offset = FP_OFF(path);
    bloodprg_adapter_trace.path_segment = FP_SEG(path);
    bloodprg_adapter_trace.handle_before = *handle;
    for (character_index = 0;
            character_index < sizeof(bloodprg_adapter_trace.path);
            ++character_index) {
        bloodprg_adapter_trace.path[character_index] = path[character_index];
    }
#endif
    success = bloodprg_dos_call_far_path(&registers, &segments, path);
    *handle = registers.x.ax;
#if defined(BLOODPRG_ADAPTER_TRACE)
    bloodprg_adapter_trace.dos_ax = registers.x.ax;
    bloodprg_adapter_trace.carry = registers.x.cflag;
    bloodprg_adapter_trace.success = success;
    bloodprg_adapter_trace.handle_after = *handle;
#endif
    return success;
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
    registers.x.bx = logical_page;
    registers.x.dx = handle;
    int86(0x67, &registers, &registers);
}

void CB_FAR cb_resource_allocation_failure(cb_u16 error_code)
{
    error_overlay_draw(error_code, (const cb_u8 CB_FAR *)0);
}

/* These names bridge far source pointers to recovered DS-relative bodies.
 * The original callers establish that segment before entry. */
cb_i16 CB_FAR pbm_image_load_and_decode_c(
        volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *file_buffer_end)
{
    return pbm_image_load_and_decode(path, file_buffer_end);
}

void CB_FAR bridge_panorama_frame_unpack_c(const cb_u8 CB_FAR *source)
{
    bridge_panorama_frame_unpack(source);
}

void CB_FAR fullscreen_copy_to_backbuffer_far(
        const cb_u32 CB_FAR *source)
{
    fullscreen_copy_to_backbuffer((const cb_u32 CB_NEAR *)source);
}

/* HIMEM.SYS uses command and result registers that have no natural C ABI. */
#if defined(__WATCOMC__)
#pragma aux cb_platform_xms_move = \
        "mov ah,0bh" \
        "call dword ptr xms_driver_entry" \
        parm [si] modify exact [ax bx cx dx]
#pragma aux cb_platform_xms_release = \
        "mov ah,0ah" \
        "call dword ptr xms_driver_entry" \
        parm [dx] modify exact [ax bx cx dx]
#pragma aux cb_platform_xms_allocate = \
        "mov ah,09h" \
        "call dword ptr xms_driver_entry" \
        "xor dh,dh" \
        "or bl,bl" \
        "jz short cb_xms_allocate_ok" \
        "inc dh" \
        "cb_xms_allocate_ok:" \
        parm [dx] value [dx ax] modify exact [ax bx cx dx]
#endif

static void cb_platform_xms_move(
        volatile bloodprg_xms_move_request CB_GAME_DATA *request);
static void cb_platform_xms_release(cb_u16 handle);
static cb_u32 cb_platform_xms_allocate(cb_u16 kilobytes);
void CB_NEAR cb_xms_move(
        volatile bloodprg_xms_move_request CB_GAME_DATA *request)
{
    cb_platform_xms_move(request);
}

void CB_NEAR cb_xms_release(cb_u16 handle)
{
    cb_platform_xms_release(handle);
}

int CB_NEAR cb_xms_allocate_kb(cb_u16 kilobytes, cb_u16 *handle)
{
    cb_u32 result = cb_platform_xms_allocate(kilobytes);

    *handle = (cb_u16)result;
    return (cb_u16)(result >> 16) == 0u;
}

void CB_NEAR cb_snd_stream_service(
        cb_u16 command,
        volatile bloodprg_snd_stream_buffer CB_GAME_DATA *buffer,
        volatile cb_u8 CB_FAR *cursor)
{
    bloodprg_snd_stream_driver_callback driver;

    driver = snd_driver_entries[6].stream;
    driver(command, buffer, cursor);
}

void CB_NEAR cb_snd_stream_play(
        cb_u16 command,
        volatile bloodprg_snd_stream_buffer CB_GAME_DATA *buffer,
        volatile cb_u8 CB_FAR *cursor)
{
    bloodprg_snd_stream_driver_callback driver;

    driver = snd_driver_entries[2].stream;
    driver(command, buffer, cursor);
}

void CB_NEAR cb_snd_clip_play(
        cb_u16 command,
        volatile bloodprg_snd_clip_descriptor CB_GAME_DATA *clip)
{
    bloodprg_snd_clip_driver_callback driver;

    driver = snd_driver_entries[2].clip;
    driver(command, clip);
}
