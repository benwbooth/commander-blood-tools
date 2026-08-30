#include <conio.h>
#include <dos.h>
#include <fcntl.h>
#include <io.h>
#include <stdio.h>

#include "xdb_alien.h"

#ifndef XDB_IMAGE_BYTES
#error XDB_IMAGE_BYTES must be defined by the source-XDB integration driver
#endif
#ifndef XDB_DATA_PARAGRAPH
#error XDB_DATA_PARAGRAPH must be defined by the source-XDB integration driver
#endif
#ifndef XDB_DATA_STATE_OFFSET
#error XDB_DATA_STATE_OFFSET must be defined by the source-XDB integration driver
#endif
#ifndef XDB_ALLOCATION_PARAGRAPHS
#define XDB_ALLOCATION_PARAGRAPHS \
    (((unsigned long)XDB_IMAGE_BYTES + 15ul) >> 4)
#endif
#ifndef XDB_RENDER_CONTINUATION_OFFSET
#error XDB_RENDER_CONTINUATION_OFFSET must be defined by the source-XDB integration driver
#endif
#ifndef XDB_RENDER_MODE_OFFSET
#error XDB_RENDER_MODE_OFFSET must be defined by the source-XDB integration driver
#endif
#if defined(XDB_DUMP_RASTER) && !defined(XDB_RASTER_STATE_OFFSET)
#error XDB_RASTER_STATE_OFFSET must be defined by the source-XDB integration driver
#endif
#if defined(XDB_DUMP_FRAME) && !defined(XDB_FINAL_CLEAR_CALL_OFFSET)
#error XDB_FINAL_CLEAR_CALL_OFFSET must be defined by the frame-oracle driver
#endif
#if defined(XDB_DUMP_FRAME) && !defined(XDB_FINAL_CLEAR_CALL_DISPLACEMENT)
#error XDB_FINAL_CLEAR_CALL_DISPLACEMENT must be defined by the frame-oracle driver
#endif

#define XDB_FILENAME "ALIEN.XDB"
#define RESULT_FILENAME "RESULT.TXT"
#define RASTER_DUMP_FILENAME "RASTER.BIN"
#define FRAME_DUMP_FILENAME "FRAME.BIN"
#define DATA_DUMP_FILENAME "DATA.BIN"
#define OBJECT_DUMP_FILENAME "OBJECT.BIN"
#define VGA_GRAPHICS_CONTROLLER_INDEX 0x03ceu
#define VGA_GRAPHICS_CONTROLLER_DATA 0x03cfu
#define VGA_SEQUENCER_INDEX 0x03c4u
#define VGA_SEQUENCER_DATA 0x03c5u
#define VGA_READ_MAP_SELECT_REGISTER 0x04u
#define VGA_PLANE_COUNT 4u
#define VGA_FRAME_PAGE_OFFSET 0x4000u
#define VGA_PLANE_FRAME_BYTES 16000u

static volatile xdb_u16 timing_scale = 7u;
static void queue_escape(void);

static void XDB_FAR test_frame_callback(xdb_u16 event, xdb_u32 clock)
{
    (void)event;
    (void)clock;
    queue_escape();
}

extern void call_overlay(
        xdb_u16 overlay_segment,
        const void XDB_NEAR *request);

#pragma aux call_overlay = \
        "push bp" \
        "push ax" \
        "xor bx,bx" \
        "push bx" \
        "mov bx,sp" \
        "mov bp,si" \
        "call dword ptr ss:[bx]" \
        "add sp,4" \
        "pop bp" \
        parm [ax] [si] \
        modify exact [ax bx cx dx si di es]

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILENAME, "w");

    if (result == NULL) {
        return 2;
    }
    fprintf(result, "%s\n", status);
    printf("%s\n", status);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static int load_overlay(xdb_u16 segment)
{
    int handle;
    unsigned error;
    unsigned long position = 0ul;

    error = _dos_open(XDB_FILENAME, O_RDONLY | O_BINARY, &handle);
    if (error != 0u) {
        return 0;
    }
    while (position < (unsigned long)XDB_IMAGE_BYTES) {
        union REGS registers;
        struct SREGS segments;
        unsigned long remaining = (unsigned long)XDB_IMAGE_BYTES - position;
        xdb_u16 count = remaining > 0xfff0ul
                ? 0xfff0u
                : (xdb_u16)remaining;

        registers.x.ax = 0x3f00u;
        registers.x.bx = (xdb_u16)handle;
        registers.x.cx = count;
        registers.x.dx = (xdb_u16)(position & 0x0ful);
        segread(&segments);
        segments.ds = (xdb_u16)(segment + (xdb_u16)(position >> 4));
        int86x(0x21, &registers, &registers, &segments);
        if (registers.x.cflag != 0u || registers.x.ax != count) {
            _dos_close(handle);
            return 0;
        }
        position += count;
    }
    _dos_close(handle);
    return 1;
}

#if defined(XDB_DUMP_RASTER) || defined(XDB_DUMP_FRAME)
static int dump_segment(const char *path, xdb_u16 segment)
{
    int handle;
    unsigned error;
    xdb_u32 position = 0ul;

    error = _dos_creat(path, 0u, &handle);
    if (error != 0u) {
        return 0;
    }
    while (position < 0x10000ul) {
        union REGS registers;
        struct SREGS segments;
        xdb_u16 count = position == 0ul ? 0xfff0u : 0x0010u;

        registers.x.ax = 0x4000u;
        registers.x.bx = (xdb_u16)handle;
        registers.x.cx = count;
        registers.x.dx = (xdb_u16)position;
        segread(&segments);
        segments.ds = segment;
        int86x(0x21, &registers, &registers, &segments);
        if (registers.x.cflag != 0u || registers.x.ax != count) {
            _dos_close(handle);
            return 0;
        }
        position += count;
    }
    _dos_close(handle);
    return 1;
}
#endif

#if defined(XDB_DUMP_FRAME)
static int preserve_rendered_frame(xdb_u16 overlay_segment)
{
    volatile xdb_u8 XDB_FAR *instruction = XDB_FAR_AT(
            volatile xdb_u8,
            overlay_segment,
            XDB_FINAL_CLEAR_CALL_OFFSET);
    xdb_u16 displacement = (xdb_u16)instruction[1]
            | ((xdb_u16)instruction[2] << 8);

    if (instruction[0] != 0xe8u
            || displacement != XDB_FINAL_CLEAR_CALL_DISPLACEMENT) {
        return 0;
    }
    instruction[0] = 0x90u;
    instruction[1] = 0x90u;
    instruction[2] = 0x90u;
    return 1;
}

static int dump_frame(void)
{
    int handle;
    unsigned error;
    xdb_u16 plane;

    error = _dos_creat(FRAME_DUMP_FILENAME, 0u, &handle);
    if (error != 0u) {
        return 0;
    }
    for (plane = 0u; plane < VGA_PLANE_COUNT; ++plane) {
        union REGS registers;
        struct SREGS segments;

        outpw(
                VGA_GRAPHICS_CONTROLLER_INDEX,
                (xdb_u16)((plane << 8) | VGA_READ_MAP_SELECT_REGISTER));
        registers.x.ax = 0x4000u;
        registers.x.bx = (xdb_u16)handle;
        registers.x.cx = VGA_PLANE_FRAME_BYTES;
        registers.x.dx = VGA_FRAME_PAGE_OFFSET;
        segread(&segments);
        segments.ds = 0xa000u;
        int86x(0x21, &registers, &registers, &segments);
        if (registers.x.cflag != 0u
                || registers.x.ax != VGA_PLANE_FRAME_BYTES) {
            _dos_close(handle);
            return 0;
        }
    }
    outpw(VGA_GRAPHICS_CONTROLLER_INDEX, VGA_READ_MAP_SELECT_REGISTER);
    _dos_close(handle);
    return 1;
}
#endif

static void set_video_mode(xdb_u8 mode)
{
    union REGS registers;

    registers.h.ah = 0u;
    registers.h.al = mode;
    int86(0x10, &registers, &registers);
}

static void configure_game_video_mode(void)
{
    volatile xdb_u16 XDB_FAR *crtc_base =
            XDB_FAR_AT(volatile xdb_u16, 0x0040u, 0x0063u);
    xdb_u16 port;
    xdb_u8 value;

    set_video_mode(0x13u);

    outp(VGA_GRAPHICS_CONTROLLER_INDEX, 5u);
    value = (xdb_u8)inp(VGA_GRAPHICS_CONTROLLER_DATA);
    outp(VGA_GRAPHICS_CONTROLLER_DATA, value & 0xefu);

    outp(VGA_GRAPHICS_CONTROLLER_INDEX, 6u);
    value = (xdb_u8)inp(VGA_GRAPHICS_CONTROLLER_DATA);
    outp(VGA_GRAPHICS_CONTROLLER_DATA, value & 0xfdu);

    outp(VGA_SEQUENCER_INDEX, 4u);
    value = (xdb_u8)inp(VGA_SEQUENCER_DATA);
    outp(VGA_SEQUENCER_DATA, (value & 0xf7u) | 0x04u);

    port = *crtc_base;
    outp(port, 0x14u);
    value = (xdb_u8)inp(port + 1u);
    outp(port + 1u, value & 0xbfu);

    outp(port, 0x17u);
    value = (xdb_u8)inp(port + 1u);
    outp(port + 1u, value | 0x40u);

    outp(port, 0x11u);
    value = (xdb_u8)inp(port + 1u);
    outp(port + 1u, value | 0x20u);
    outpw(VGA_SEQUENCER_INDEX, 0x0f02u);
}

static void queue_escape(void)
{
    volatile xdb_u16 XDB_FAR *head = XDB_FAR_AT(xdb_u16, 0x0040u, 0x001au);
    volatile xdb_u16 XDB_FAR *tail = XDB_FAR_AT(xdb_u16, 0x0040u, 0x001cu);
    volatile xdb_u16 XDB_FAR *first = XDB_FAR_AT(xdb_u16, 0x0040u, 0x001eu);

    *head = 0x001eu;
    *tail = 0x0020u;
    *first = 0x011bu;
}

int main(void)
{
    xdb_alien_api_request request;
    volatile xdb_alien_segment_directory XDB_FAR *directory;
    xdb_u16 overlay_segment;
    xdb_u16 data_segment;
    xdb_u16 expected_object;
    xdb_u16 expected_palette;
    xdb_u16 expected_raster;
#if defined(XDB_DUMP_RASTER)
    xdb_u16 published_raster;
#endif
    xdb_u16 paragraphs = (xdb_u16)XDB_ALLOCATION_PARAGRAPHS;
    int status = 0;

    if (_dos_allocmem(paragraphs, &overlay_segment) != 0u) {
        return write_result("FAIL source alien allocation");
    }
    if (!load_overlay(overlay_segment)) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source alien load");
    }
#if defined(XDB_DUMP_FRAME)
    if (!preserve_rendered_frame(overlay_segment)) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL alien frame cleanup patch");
    }
#endif

    request.timing_scale = XDB_FAR_AT(
            volatile xdb_u16,
            FP_SEG(&timing_scale),
            FP_OFF(&timing_scale));
    request.frame_callback = test_frame_callback;
    configure_game_video_mode();
    queue_escape();
    call_overlay(overlay_segment, &request);
#if defined(XDB_DUMP_FRAME)
    if (!dump_frame()) {
        set_video_mode(0x03u);
        _dos_freemem(overlay_segment);
        return write_result("FAIL alien frame dump");
    }
#endif
    set_video_mode(0x03u);

    data_segment = (xdb_u16)(overlay_segment + XDB_DATA_PARAGRAPH);
    directory = XDB_FAR_AT(
            volatile xdb_alien_segment_directory,
            data_segment,
            0u);
    expected_object = (xdb_u16)(data_segment + directory->object_segment_delta);
    expected_palette = (xdb_u16)(expected_object + directory->palette_segment_delta);
    expected_raster = (xdb_u16)(expected_palette + directory->raster_segment_delta);
#if defined(XDB_DUMP_RASTER)
    published_raster = *XDB_FAR_AT(
            volatile xdb_u16,
            overlay_segment,
            XDB_RASTER_STATE_OFFSET);
    if (!dump_segment(RASTER_DUMP_FILENAME, published_raster)) {
        status = write_result("FAIL source alien raster dump");
    } else
#endif
#if defined(XDB_DUMP_FRAME)
    if (!dump_segment(DATA_DUMP_FILENAME, data_segment)) {
        status = write_result("FAIL alien data dump");
    } else if (!dump_segment(OBJECT_DUMP_FILENAME, expected_object)) {
        status = write_result("FAIL alien object dump");
    } else if (!dump_segment(RASTER_DUMP_FILENAME, expected_raster)) {
        status = write_result("FAIL alien raster dump");
    } else
#endif
    if (*XDB_FAR_AT(
                volatile xdb_u16,
                overlay_segment,
                XDB_DATA_STATE_OFFSET) != data_segment) {
        status = write_result("FAIL source alien data publication");
    } else if (directory->object_segment != expected_object
            || directory->palette_segment != expected_palette
            || directory->raster_segment != expected_raster) {
        status = write_result("FAIL source alien segment directory");
    } else if (*XDB_FAR_AT(
                volatile xdb_u16,
                expected_raster,
                XDB_RENDER_CONTINUATION_OFFSET) != XDB_RENDER_MODE_OFFSET) {
        status = write_result("FAIL source alien render continuation");
    } else if (timing_scale != 7u) {
        status = write_result("FAIL source alien timing writeback");
    } else {
        status = write_result("PASS source-linked alien XDB");
    }

    _dos_freemem(overlay_segment);
    return status;
}
