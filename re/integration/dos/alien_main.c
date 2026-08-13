#include <dos.h>
#include <stdio.h>

#include "xdb_alien.h"
#include "xdb_mouse.h"
#include "xdb_video.h"

#define RESULT_FILE "RESULT.TXT"
#define CALL_CAPACITY 16u

enum call_id {
    CALL_VGA_CLEAR = 1,
    CALL_MOUSE_BOUNDS,
    CALL_MOUSE_POSITION,
    CALL_MOUSE_CAMERA,
    CALL_CAMERA_MATRIX,
    CALL_PRIMARY_MESH,
    CALL_STARFIELD,
    CALL_METHOD,
    CALL_TRANSFORM,
    CALL_BUCKET_FACES
};

volatile xdb_u16 XDB_CODE_DATA xdb_croolis_data_segment;
volatile xdb_u16 XDB_CODE_DATA xdb_alien_key_event;

volatile xdb_u32 xdb_alien_frame_clock = 0x10203040ul;
volatile xdb_u32 xdb_alien_last_callback_clock;
volatile xdb_u16 xdb_alien_callback_countdown = 3u;
xdb_alien_frame_callback xdb_alien_frame_callback_ptr;
volatile xdb_u16 xdb_video_page_4000 = 0x4000u;
volatile xdb_u16 xdb_alien_framebuffer_segment = 0xa400u;
volatile xdb_u16 xdb_alien_exit_requested = 0xa55au;
volatile xdb_u16 xdb_alien_frame_state = 0x1357u;
volatile xdb_i16 xdb_alien_view_x;
volatile xdb_i16 xdb_alien_view_y;
volatile xdb_i16 xdb_alien_view_z;
volatile xdb_i16 xdb_alien_camera_pitch;
volatile xdb_i16 xdb_alien_camera_pan;
volatile xdb_i16 xdb_alien_camera_pan_secondary;
volatile xdb_i16 xdb_alien_camera_depth_step;
volatile xdb_u16 xdb_alien_control_latch = 0x2468u;
volatile xdb_alien_projection_context XDB_NEAR
        *xdb_alien_active_projection_context;
volatile xdb_u8 xdb_alien_method_table[4];
const volatile xdb_u8 xdb_alien_display_palette[768] = {0};

static xdb_alien_method_context contexts[2];
volatile xdb_u16 xdb_alien_render_context_offsets[3];
static xdb_u8 calls[CALL_CAPACITY];
static xdb_u16 call_count;
static xdb_u16 callback_count;
static xdb_u16 callback_event;
static xdb_u32 callback_clock;
static xdb_u16 initial_data_segment;

static void record_call(xdb_u8 id)
{
    if (call_count < CALL_CAPACITY) {
        calls[call_count++] = id;
    }
}

static xdb_u16 capture_data_segment(void);
#pragma aux capture_data_segment = \
        "mov ax,ds" \
        value [ax] \
        modify exact []

static xdb_u16 queue_escape(void);
#pragma aux queue_escape = \
        "mov ah,5" \
        "mov cx,011bh" \
        "int 16h" \
        "xor ah,ah" \
        value [ax] \
        modify exact [ax cx]

static void XDB_NEAR test_method(
        xdb_alien_method_context XDB_NEAR *context)
{
    if (context == &contexts[0] || context == &contexts[1]) {
        record_call(CALL_METHOD);
    }
}
#pragma aux test_method parm [di]

static xdb_u32 capture_callback_clock(void);
#pragma aux capture_callback_clock = \
        "mov eax,edx" \
        "shr edx,16" \
        value [dx ax] \
        modify exact [ax dx]

static xdb_u16 capture_callback_event(void);
#pragma aux capture_callback_event = value [ax] modify exact []

static void XDB_FAR test_frame_callback(void)
{
    callback_event = capture_callback_event();
    callback_clock = capture_callback_clock();
    ++callback_count;
}

void XDB_NEAR xdb_croolis_vga_clear_and_sync(void)
{
    record_call(CALL_VGA_CLEAR);
}

void XDB_NEAR xdb_croolis_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y)
{
    if ((call_count == 1u && max_x == 640u && max_y == 1024u)
            || (call_count == 13u && max_x == 3000u && max_y == 200u)) {
        record_call(CALL_MOUSE_BOUNDS);
    }
}

void XDB_NEAR xdb_croolis_mouse_position_set(xdb_u16 x, xdb_u16 y)
{
    if (x == 320u && y == 512u) {
        record_call(CALL_MOUSE_POSITION);
    }
}

void XDB_NEAR xdb_croolis_mouse_camera_step(void)
{
    record_call(CALL_MOUSE_CAMERA);
}

void XDB_NEAR xdb_croolis_camera_matrix_update(void)
{
    record_call(CALL_CAMERA_MATRIX);
}

void XDB_NEAR xdb_croolis_project_primary_mesh_then_render(void)
{
    record_call(CALL_PRIMARY_MESH);
}

void XDB_NEAR xdb_croolis_render_starfield(void)
{
    record_call(CALL_STARFIELD);
}

void XDB_NEAR xdb_croolis_transform_and_project(void)
{
    record_call(CALL_TRANSFORM);
}

void XDB_NEAR xdb_croolis_bucket_faces_then_render(void)
{
    record_call(CALL_BUCKET_FACES);
}

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) {
        return 2;
    }
    fprintf(result, "%s\n", status);
    printf("%s\n", status);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static int calls_match(void)
{
    static const xdb_u8 expected[] = {
        CALL_VGA_CLEAR,
        CALL_MOUSE_BOUNDS,
        CALL_MOUSE_POSITION,
        CALL_MOUSE_CAMERA,
        CALL_CAMERA_MATRIX,
        CALL_PRIMARY_MESH,
        CALL_STARFIELD,
        CALL_METHOD,
        CALL_TRANSFORM,
        CALL_METHOD,
        CALL_TRANSFORM,
        CALL_BUCKET_FACES,
        CALL_VGA_CLEAR,
        CALL_MOUSE_BOUNDS,
    };
    xdb_u16 index;

    if (call_count != sizeof(expected)) {
        return 0;
    }
    for (index = 0u; index != sizeof(expected); ++index) {
        if (calls[index] != expected[index]) {
            return 0;
        }
    }
    return 1;
}

int main(void)
{
    xdb_alien_method_callback XDB_NEAR *methods =
            (xdb_alien_method_callback XDB_NEAR *)xdb_alien_method_table;

    initial_data_segment = capture_data_segment();
    xdb_croolis_data_segment = initial_data_segment;
    contexts[0].method_table_offset = 0u;
    contexts[1].method_table_offset = 2u;
    methods[0] = test_method;
    methods[1] = test_method;
    xdb_alien_render_context_offsets[0] = (xdb_u16)&contexts[0];
    xdb_alien_render_context_offsets[1] = (xdb_u16)&contexts[1];
    xdb_alien_render_context_offsets[2] = 0u;
    xdb_alien_frame_callback_ptr =
            (xdb_alien_frame_callback)test_frame_callback;

    if (queue_escape() != 0u) {
        return write_result("FAIL keyboard queue");
    }
    xdb_croolis_main();

    if (capture_data_segment() != initial_data_segment) {
        return write_result("FAIL data segment restore");
    }
    if (!calls_match()) {
        xdb_u16 index;

        printf("call count %u:", call_count);
        for (index = 0u; index != call_count; ++index) {
            printf(" %u", calls[index]);
        }
        printf("\n");
        return write_result("FAIL call order");
    }
    if (callback_count != 1u
            || callback_event != 2u
            || callback_clock != 0x10203048ul) {
        return write_result("FAIL timer callback");
    }
    if (xdb_alien_frame_clock != 0x10203048ul
            || xdb_alien_last_callback_clock != 0x10203048ul
            || xdb_alien_callback_countdown != 0u) {
        return write_result("FAIL timer state");
    }
    if (xdb_video_page_4000 != 0x8000u
            || xdb_alien_framebuffer_segment != 0xa800u) {
        return write_result("FAIL page rotation");
    }
    if (xdb_alien_frame_state != 0u
            || xdb_alien_view_x != 0x075d
            || (xdb_u16)xdb_alien_view_y != 0xff11u
            || (xdb_u16)xdb_alien_view_z != 0xd9c2u
            || xdb_alien_camera_pitch != 0
            || xdb_alien_camera_pan != 0x0678
            || xdb_alien_camera_pan_secondary != 0
            || xdb_alien_camera_depth_step != 0) {
        return write_result("FAIL initial state");
    }
    if (xdb_alien_control_latch != 0u
            || xdb_alien_exit_requested != 0u
            || xdb_alien_key_event != 0x011bu
            || xdb_alien_active_projection_context
                    != (xdb_alien_projection_context XDB_NEAR *)&contexts[1]) {
        return write_result("FAIL loop state");
    }
    return write_result("PASS alien main");
}
