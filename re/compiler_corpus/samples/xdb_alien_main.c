/* Codegen probe for the alien overlay's far main-loop owner. */
#include <dos.h>
#include <string.h>

typedef unsigned char xdb_u8;
typedef unsigned int xdb_u16;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;

#define XDB_FAR far
#define XDB_NEAR near
#define XDB_FAR_AT(type, segment, offset) \
    ((type XDB_FAR *)MK_FP((segment), (offset)))
#if defined(__WATCOMC__)
#define XDB_CODE_DATA __based(__segname("_CODE"))
#else
#define XDB_CODE_DATA XDB_FAR
#endif
#define XDB_ALIEN_PALETTE_SIZE 0x0300u
#define XDB_ALIEN_FRAMEBUFFER_SIZE 0x3e80u

typedef struct xdb_alien_method_context {
    xdb_u8 field_000[0x34];
    xdb_u16 method_table_offset;
} xdb_alien_method_context;

typedef struct xdb_alien_projection_context xdb_alien_projection_context;
typedef void XDB_NEAR xdb_alien_method_function(
        xdb_alien_method_context XDB_NEAR *context);
typedef xdb_alien_method_function XDB_NEAR *xdb_alien_method_callback;
typedef void XDB_FAR xdb_alien_frame_function(
        xdb_u16 event,
        xdb_u32 clock);
typedef xdb_alien_frame_function XDB_FAR *xdb_alien_frame_callback;

typedef union xdb_video_page {
    xdb_u16 word;
    struct {
        xdb_u8 low;
        xdb_u8 high;
    } byte;
} xdb_video_page;

extern volatile xdb_u16 XDB_CODE_DATA xdb_croolis_data_segment;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_key_event;
extern volatile xdb_u32 xdb_alien_frame_clock;
extern volatile xdb_u32 xdb_alien_last_callback_clock;
extern volatile xdb_u16 xdb_alien_callback_countdown;
extern xdb_alien_frame_callback xdb_alien_frame_callback_ptr;
extern volatile xdb_u16 xdb_video_page_4000;
extern volatile xdb_u16 xdb_alien_framebuffer_segment;
extern volatile xdb_u16 xdb_alien_exit_requested;
extern volatile xdb_u16 xdb_alien_frame_state;
extern volatile xdb_i16 xdb_alien_view_x;
extern volatile xdb_i16 xdb_alien_view_y;
extern volatile xdb_i16 xdb_alien_view_z;
extern volatile xdb_i16 xdb_alien_camera_pitch;
extern volatile xdb_i16 xdb_alien_camera_pan;
extern volatile xdb_i16 xdb_alien_camera_pan_secondary;
extern volatile xdb_i16 xdb_alien_camera_depth_step;
extern volatile xdb_u16 xdb_alien_control_latch;
extern volatile xdb_u16 xdb_alien_render_context_offsets[];
extern volatile xdb_alien_projection_context XDB_NEAR
        *xdb_alien_active_projection_context;
extern volatile xdb_u8 xdb_alien_method_table[];
extern const volatile xdb_u8 xdb_alien_display_palette[768];

extern void XDB_NEAR xdb_port_write_u8(xdb_u16 port, xdb_u8 value);
#pragma aux xdb_port_write_u8 = \
        "out dx,al" \
        parm [dx] [al] \
        modify exact []
extern void XDB_NEAR xdb_port_write_u16(xdb_u16 port, xdb_u16 value);
#pragma aux xdb_port_write_u16 = \
        "out dx,ax" \
        parm [dx] [ax] \
        modify exact []
extern void XDB_NEAR xdb_port_write_buffer_u8(
        xdb_u16 port,
        const volatile xdb_u8 XDB_NEAR *source,
        xdb_u16 count);
#pragma aux xdb_port_write_buffer_u8 = \
        "rep outsb" \
        parm [dx] [si] [cx] \
        modify exact [cx si]
extern void XDB_NEAR xdb_direction_forward(void);
#pragma aux xdb_direction_forward = "cld" modify exact []
extern xdb_u16 XDB_NEAR xdb_keyboard_ready(void);
#pragma aux xdb_keyboard_ready = \
        "mov ah,1" \
        "int 16h" \
        "setnz al" \
        "xor ah,ah" \
        value [ax] \
        modify exact [ax]
extern xdb_u16 XDB_NEAR xdb_keyboard_read(void);
#pragma aux xdb_keyboard_read = \
        "xor ah,ah" \
        "int 16h" \
        value [ax] \
        modify exact [ax]
extern void XDB_NEAR xdb_alien_frame_callback_invoke(
        xdb_u16 event,
        xdb_u32 clock);
#pragma aux xdb_alien_frame_callback_invoke = \
        "shl edx,16" \
        "mov dx,ax" \
        "mov ax,bx" \
        "call dword ptr xdb_alien_frame_callback_ptr" \
        parm [bx] [dx ax] \
        modify exact [ax bx cx dx si di bp es]
extern xdb_u16 XDB_NEAR xdb_alien_data_segments_install(
        xdb_u16 data_segment);
#pragma aux xdb_alien_data_segments_install = \
        "mov dx,ds" \
        "mov ds,ax" \
        "mov es,ax" \
        "mov fs,ax" \
        parm [ax] \
        value [dx] \
        modify exact [ax]
extern void XDB_NEAR xdb_alien_data_segment_restore(xdb_u16 data_segment);
#pragma aux xdb_alien_data_segment_restore = \
        "mov ds,ax" \
        parm [ax] \
        modify exact []

extern void XDB_NEAR xdb_croolis_vga_clear_and_sync(void);
extern void XDB_NEAR xdb_croolis_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y);
extern void XDB_NEAR xdb_croolis_mouse_position_set(xdb_u16 x, xdb_u16 y);
extern void XDB_NEAR xdb_croolis_mouse_camera_step(void);
extern void XDB_NEAR xdb_croolis_camera_matrix_update(void);
extern void XDB_NEAR xdb_croolis_project_primary_mesh_then_render(void);
extern void XDB_NEAR xdb_croolis_render_starfield(void);
extern void XDB_NEAR xdb_croolis_transform_and_project(void);
extern void XDB_NEAR xdb_croolis_bucket_faces_then_render(void);

#pragma aux xdb_alien_method_function \
        parm [di] modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_vga_clear_and_sync \
        modify exact [ax bx cx dx di es]
#pragma aux xdb_croolis_mouse_bounds_set \
        parm [cx] [dx] modify exact [ax cx dx]
#pragma aux xdb_croolis_mouse_position_set parm [cx] [dx] modify exact [ax]
#pragma aux xdb_croolis_mouse_camera_step modify exact [ax bx cx dx]
#pragma aux xdb_croolis_camera_matrix_update \
        modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_project_primary_mesh_then_render \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_render_starfield \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_transform_and_project \
        modify exact [ax bx cx dx si di bp es]
#pragma aux xdb_croolis_bucket_faces_then_render \
        modify exact [ax bx cx dx si di bp es]

void XDB_FAR xdb_alien_main_probe(void);
#pragma aux xdb_alien_main_probe modify exact [ax bx cx dx si di bp es]

void XDB_FAR xdb_alien_main_probe(void)
{
    xdb_u16 saved_data_segment = xdb_alien_data_segments_install(
            xdb_croolis_data_segment);
    xdb_u16 running = 1u;

    xdb_alien_exit_requested = 0u;
    xdb_croolis_vga_clear_and_sync();
    xdb_port_write_u8(0x03c8u, 0u);
    xdb_port_write_buffer_u8(
            0x03c9u,
            xdb_alien_display_palette,
            XDB_ALIEN_PALETTE_SIZE);
    xdb_croolis_mouse_bounds_set(640u, 1024u);
    xdb_croolis_mouse_position_set(320u, 512u);

    xdb_alien_frame_state = 0u;
    xdb_alien_view_x = 0x075d;
    xdb_alien_view_y = (xdb_i16)0xff11u;
    xdb_alien_view_z = (xdb_i16)0xd9c2u;
    xdb_alien_camera_pitch = 0;
    xdb_alien_camera_pan = 0x0678;
    xdb_alien_camera_pan_secondary = 0;
    xdb_alien_camera_depth_step = 0;
    xdb_alien_last_callback_clock = xdb_alien_frame_clock - 620ul;

    while (running != 0u) {
        volatile xdb_u16 XDB_FAR *framebuffer = XDB_FAR_AT(
                volatile xdb_u16,
                xdb_alien_framebuffer_segment,
                0u);
        volatile xdb_u16 XDB_NEAR *context_offset;
        xdb_video_page page;
        xdb_u16 display_page;
        xdb_i16 event;
        xdb_u32 clock;

        xdb_direction_forward();
        xdb_port_write_u16(0x03c4u, 0x0f02u);
        _fmemset((void XDB_FAR *)framebuffer, 0, XDB_ALIEN_FRAMEBUFFER_SIZE);
        xdb_croolis_mouse_camera_step();
        xdb_croolis_camera_matrix_update();
        xdb_croolis_project_primary_mesh_then_render();
        xdb_croolis_render_starfield();

        context_offset = xdb_alien_render_context_offsets;
        do {
            xdb_alien_method_context XDB_NEAR *context =
                    (xdb_alien_method_context XDB_NEAR *)*context_offset;

            ++context_offset;
            xdb_alien_active_projection_context =
                    (xdb_alien_projection_context XDB_NEAR *)context;
            (*(xdb_alien_method_callback XDB_NEAR *)(
                    xdb_alien_method_table
                    + context->method_table_offset))(context);
            xdb_croolis_transform_and_project();
        } while (*context_offset != 0u);

        xdb_alien_control_latch = 0u;
        xdb_croolis_bucket_faces_then_render();

        page.word = xdb_video_page_4000;
        display_page = page.word;
        page.byte.high = (xdb_u8)(page.byte.high + 0x40u);
        xdb_video_page_4000 = page.word;
        xdb_port_write_u16(
                0x03d4u,
                (xdb_u16)((display_page & 0xff00u) | 0x000cu));
        page.byte.high = (xdb_u8)((page.byte.high >> 4) | 0xa0u);
        xdb_alien_framebuffer_segment = page.word;

        if (xdb_alien_exit_requested != 0u) {
            break;
        }

        xdb_alien_frame_clock += 8ul;
        event = (xdb_i16)(xdb_alien_callback_countdown - 1u);
        xdb_alien_callback_countdown = 0u;
        clock = xdb_alien_frame_clock;
        if (event >= 0) {
            xdb_alien_frame_callback_invoke((xdb_u16)event, clock);
            xdb_alien_last_callback_clock = clock;
        } else if (clock - xdb_alien_last_callback_clock >= 600ul) {
            xdb_u32 callback_clock = clock - 1000ul;

            if (xdb_alien_control_latch != 0u) {
                xdb_alien_frame_callback_invoke(2u, clock);
                callback_clock = clock;
            }
            xdb_alien_last_callback_clock = callback_clock;
        }

        while (xdb_keyboard_ready() != 0u) {
            xdb_u16 key = xdb_keyboard_read();
            xdb_u8 character = (xdb_u8)key;

            xdb_alien_key_event = key;
            if (character == 'p' || character == 'P') {
                do {
                    key = xdb_keyboard_read();
                    character = (xdb_u8)key;
                } while (character != 'p' && character != 'P');
                break;
            }
            if (character == 0x1bu) {
                running = 0u;
                break;
            }
        }
    }

    xdb_croolis_vga_clear_and_sync();
    xdb_croolis_mouse_bounds_set(3000u, 200u);
    xdb_alien_data_segment_restore(saved_data_segment);
}
