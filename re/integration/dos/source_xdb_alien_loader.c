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
#ifndef XDB_RENDER_CONTINUATION_OFFSET
#error XDB_RENDER_CONTINUATION_OFFSET must be defined by the source-XDB integration driver
#endif
#ifndef XDB_RENDER_MODE_OFFSET
#error XDB_RENDER_MODE_OFFSET must be defined by the source-XDB integration driver
#endif
#if defined(XDB_DUMP_RASTER) && !defined(XDB_RASTER_STATE_OFFSET)
#error XDB_RASTER_STATE_OFFSET must be defined by the source-XDB integration driver
#endif

#define XDB_FILENAME "ALIEN.XDB"
#define RESULT_FILENAME "RESULT.TXT"
#define RASTER_DUMP_FILENAME "RASTER.BIN"

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

#if defined(XDB_DUMP_RASTER)
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

static void set_video_mode(xdb_u8 mode)
{
    union REGS registers;

    registers.h.ah = 0u;
    registers.h.al = mode;
    int86(0x10, &registers, &registers);
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
    xdb_u16 paragraphs = (xdb_u16)(
            ((unsigned long)XDB_IMAGE_BYTES + 15ul) >> 4);
    int status = 0;

    if (_dos_allocmem(paragraphs, &overlay_segment) != 0u) {
        return write_result("FAIL source alien allocation");
    }
    if (!load_overlay(overlay_segment)) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source alien load");
    }

    request.timing_scale = XDB_FAR_AT(
            volatile xdb_u16,
            FP_SEG(&timing_scale),
            FP_OFF(&timing_scale));
    request.frame_callback = test_frame_callback;
    set_video_mode(0x13u);
    queue_escape();
    call_overlay(overlay_segment, &request);
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
