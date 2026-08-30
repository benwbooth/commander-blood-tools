#include <conio.h>
#include <dos.h>
#include <fcntl.h>
#include <io.h>
#include <stdio.h>
#include <stdlib.h>

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
#if defined(XDB_DUMP_FRAME) && !defined(XDB_CALLBACK_LOAD_OFFSET)
#error XDB_CALLBACK_LOAD_OFFSET must be defined by the frame-oracle driver
#endif

#define XDB_FILENAME "ALIEN.XDB"
#define RESULT_FILENAME "RESULT.TXT"
#define RASTER_DUMP_FILENAME "RASTER.BIN"
#define FRAME_DUMP_FILENAME "FRAME.BIN"
#define DATA_DUMP_FILENAME "DATA.BIN"
#define OBJECT_DUMP_FILENAME "OBJECT.BIN"
#define PALETTE_DUMP_FILENAME "PALETTE.BIN"
#define VGA_GRAPHICS_CONTROLLER_INDEX 0x03ceu
#define VGA_GRAPHICS_CONTROLLER_DATA 0x03cfu
#define VGA_SEQUENCER_INDEX 0x03c4u
#define VGA_SEQUENCER_DATA 0x03c5u
#define VGA_DAC_READ_INDEX 0x03c7u
#define VGA_DAC_DATA 0x03c9u
#define VGA_READ_MAP_SELECT_REGISTER 0x04u
#define VGA_CRTC_START_HIGH_REGISTER 0x0cu
#define VGA_CRTC_START_LOW_REGISTER 0x0du
#define VGA_PLANE_COUNT 4u
#define VGA_PLANE_FRAME_BYTES 16000u
#define VGA_PALETTE_BYTES 768u
#define INPUT_CAMPAIGN_CENTERED 0u
#define INPUT_CAMPAIGN_CORNERS 1u
#define ALIEN_MOUSE_CENTER_X 320u
#define ALIEN_MOUSE_CENTER_Y 512u
#define ALIEN_MOUSE_MAXIMUM_X 640u
#define ALIEN_MOUSE_MAXIMUM_Y 1024u
#define INPUT_CAMPAIGN_PHASE_MASK 7u
#define INPUT_CAMPAIGN_LEFT_PHASE 1u
#define INPUT_CAMPAIGN_RIGHT_PHASE 2u
#define INPUT_CAMPAIGN_TOP_PHASE 3u
#define INPUT_CAMPAIGN_BOTTOM_PHASE 4u
#define INPUT_CAMPAIGN_TOP_LEFT_PHASE 5u
#define INPUT_CAMPAIGN_BOTTOM_RIGHT_PHASE 6u

static volatile xdb_u16 timing_scale = 7u;
static void queue_escape(void);
#if defined(XDB_DUMP_FRAME)
static xdb_u16 capture_frame_count = 1u;
static xdb_u16 rendered_frame_count;
static xdb_u16 input_campaign = INPUT_CAMPAIGN_CENTERED;
static xdb_u16 XDB_CODE_DATA callback_data_segment;

static int parse_word_argument(
        const char *text,
        xdb_u16 *value,
        int allow_zero)
{
    char *end;
    unsigned long parsed = strtoul(text, &end, 10);

    if (*text == '\0'
            || *end != '\0'
            || parsed > 0xfffful
            || (!allow_zero && parsed == 0ul)) {
        return 0;
    }
    *value = (xdb_u16)parsed;
    return 1;
}

static xdb_u16 data_segment_install(xdb_u16 segment);

#pragma aux data_segment_install = \
        "mov ax,ds" \
        "mov ds,dx" \
        parm [dx] \
        value [ax] \
        modify exact []

static void mouse_position_set(xdb_u16 x, xdb_u16 y);

#pragma aux mouse_position_set = \
        "mov ax,4" \
        "int 33h" \
        parm [cx] [dx] \
        modify exact [ax bx]

static void advance_capture_mouse(void)
{
    xdb_u16 phase = (xdb_u16)(
            rendered_frame_count & INPUT_CAMPAIGN_PHASE_MASK);
    xdb_u16 x = ALIEN_MOUSE_CENTER_X;
    xdb_u16 y = ALIEN_MOUSE_CENTER_Y;

    if (phase == INPUT_CAMPAIGN_LEFT_PHASE
            || phase == INPUT_CAMPAIGN_TOP_LEFT_PHASE) {
        x = 0u;
    } else if (phase == INPUT_CAMPAIGN_RIGHT_PHASE
            || phase == INPUT_CAMPAIGN_BOTTOM_RIGHT_PHASE) {
        x = ALIEN_MOUSE_MAXIMUM_X;
    }
    if (phase == INPUT_CAMPAIGN_TOP_PHASE
            || phase == INPUT_CAMPAIGN_TOP_LEFT_PHASE) {
        y = 0u;
    } else if (phase == INPUT_CAMPAIGN_BOTTOM_PHASE
            || phase == INPUT_CAMPAIGN_BOTTOM_RIGHT_PHASE) {
        y = ALIEN_MOUSE_MAXIMUM_Y;
    }
    mouse_position_set(x, y);
}
#endif

static void XDB_FAR test_frame_callback(xdb_u16 event, xdb_u32 clock)
{
#if defined(XDB_DUMP_FRAME)
    xdb_u16 saved_data_segment = data_segment_install(callback_data_segment);
    int capture_complete;
#endif

    (void)event;
    (void)clock;
#if defined(XDB_DUMP_FRAME)
    ++rendered_frame_count;
    capture_complete = rendered_frame_count >= capture_frame_count;
    if (!capture_complete) {
        if (input_campaign == INPUT_CAMPAIGN_CORNERS) {
            advance_capture_mouse();
        }
    }
    data_segment_install(saved_data_segment);
    if (!capture_complete) {
        return;
    }
#endif
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
    volatile xdb_u8 XDB_FAR *callback_load = XDB_FAR_AT(
            volatile xdb_u8,
            overlay_segment,
            XDB_CALLBACK_LOAD_OFFSET);

    if (instruction[0] != 0xe8u
            || displacement != XDB_FINAL_CLEAR_CALL_DISPLACEMENT
            || callback_load[0] != 0xa1u
            || callback_load[1] != 0x1eu
            || callback_load[2] != 0x00u) {
        return 0;
    }
    instruction[0] = 0x90u;
    instruction[1] = 0x90u;
    instruction[2] = 0x90u;
    callback_load[0] = 0xb8u;
    callback_load[1] = 0x01u;
    callback_load[2] = 0x00u;
    return 1;
}

static int dump_frame(void)
{
    int handle;
    unsigned error;
    xdb_u16 plane;
    volatile xdb_u16 XDB_FAR *crtc_base =
            XDB_FAR_AT(volatile xdb_u16, 0x0040u, 0x0063u);
    xdb_u16 crtc_port = *crtc_base;
    xdb_u16 frame_page_offset;

    outp(crtc_port, VGA_CRTC_START_HIGH_REGISTER);
    frame_page_offset = (xdb_u16)((xdb_u16)inp(crtc_port + 1u) << 8);
    outp(crtc_port, VGA_CRTC_START_LOW_REGISTER);
    frame_page_offset |= (xdb_u16)inp(crtc_port + 1u);

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
        registers.x.dx = frame_page_offset;
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

static int dump_palette(void)
{
    xdb_u8 palette[VGA_PALETTE_BYTES];
    int handle;
    unsigned error;
    unsigned index;
    unsigned written;

    outp(VGA_DAC_READ_INDEX, 0u);
    for (index = 0u; index < VGA_PALETTE_BYTES; ++index) {
        palette[index] = (xdb_u8)inp(VGA_DAC_DATA);
    }
    error = _dos_creat(PALETTE_DUMP_FILENAME, 0u, &handle);
    if (error != 0u) {
        return 0;
    }
    error = _dos_write(
            handle,
            palette,
            VGA_PALETTE_BYTES,
            &written);
    _dos_close(handle);
    return error == 0u && written == VGA_PALETTE_BYTES;
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

int main(int argc, char **argv)
{
    xdb_alien_api_request request;
    volatile xdb_alien_segment_directory XDB_FAR *directory;
    xdb_u16 overlay_segment;
    xdb_u16 data_segment;
    xdb_u16 expected_object;
    xdb_u16 expected_palette;
    xdb_u16 expected_raster;
    xdb_u16 expected_timing_scale;
#if defined(XDB_DUMP_RASTER)
    xdb_u16 published_raster;
#endif
    xdb_u16 paragraphs = (xdb_u16)XDB_ALLOCATION_PARAGRAPHS;
    int status = 0;

#if defined(XDB_DUMP_FRAME)
    if (argc >= 2) {
        xdb_u16 requested_frame_count;

        if (!parse_word_argument(argv[1], &requested_frame_count, 0)) {
            return write_result("FAIL alien frame checkpoint");
        }
        capture_frame_count = requested_frame_count;
    }
    if (argc >= 3) {
        xdb_u16 requested_timing_scale;

        if (!parse_word_argument(argv[2], &requested_timing_scale, 1)) {
            return write_result("FAIL alien timing argument");
        }
        timing_scale = requested_timing_scale;
    }
    if (argc >= 4) {
        xdb_u16 requested_input_campaign;

        if (!parse_word_argument(argv[3], &requested_input_campaign, 1)
                || requested_input_campaign > INPUT_CAMPAIGN_CORNERS) {
            return write_result("FAIL alien input campaign");
        }
        input_campaign = requested_input_campaign;
    }
    if (argc > 4) {
        return write_result("FAIL alien frame arguments");
    }
#else
    (void)argc;
    (void)argv;
#endif
    expected_timing_scale = timing_scale;
#if defined(XDB_DUMP_FRAME)
    callback_data_segment = FP_SEG(&timing_scale);
#endif

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
#if !defined(XDB_DUMP_FRAME) || defined(XDB_PREQUEUE_ESCAPE)
    queue_escape();
#endif
    call_overlay(overlay_segment, &request);
#if defined(XDB_DUMP_FRAME) && !defined(XDB_PREQUEUE_ESCAPE)
    if (rendered_frame_count < capture_frame_count) {
        set_video_mode(0x03u);
        _dos_freemem(overlay_segment);
        return write_result("FAIL alien frame not reached");
    }
#endif
#if defined(XDB_DUMP_FRAME)
    if (!dump_frame()) {
        set_video_mode(0x03u);
        _dos_freemem(overlay_segment);
        return write_result("FAIL alien frame dump");
    }
    if (!dump_palette()) {
        set_video_mode(0x03u);
        _dos_freemem(overlay_segment);
        return write_result("FAIL alien palette dump");
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
    } else if (timing_scale != expected_timing_scale) {
        status = write_result("FAIL source alien timing writeback");
    } else {
        status = write_result("PASS source-linked alien XDB");
    }

    _dos_freemem(overlay_segment);
    return status;
}
