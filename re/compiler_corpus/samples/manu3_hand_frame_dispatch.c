/* Codegen probe for BLOODPRG 0x001610. */

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;

#define FAR far
#define NEAR near

typedef struct cursor_position_probe {
    i16 x;
    i16 y;
} cursor_position_probe;

typedef struct manu3_api_request_probe {
    cursor_position_probe cursor;
    u16 animation_selector;
    u16 framebuffer_window_offset;
} manu3_api_request_probe;

typedef void (FAR *manu3_entry_probe)(
        const volatile manu3_api_request_probe FAR *request);

extern volatile u8 presentation_mode_probe;
extern volatile u8 hud_mode_probe;
extern volatile u8 scene_dispatch_blocked_probe;
extern volatile u8 presentation_request_flags_probe;
extern volatile u8 frame_delay_probe;
extern volatile i16 mouse_x_probe;
extern volatile i16 mouse_y_probe;
extern volatile i16 graphics_draw_page_offset_probe;
extern volatile u16 animation_selector_request_probe;
extern volatile u16 animation_selector_current_probe;
extern volatile manu3_api_request_probe api_request_probe;
extern manu3_entry_probe overlay_entry_probe;

void NEAR manu3_hand_frame_dispatch_probe(void)
{
    u16 selector;

    if ((presentation_mode_probe & 1u) != 0u
            || (hud_mode_probe & 1u) != 0u) {
        return;
    }

    selector = animation_selector_request_probe;
    if ((i16)selector < 0) {
        return;
    }

    if (selector == animation_selector_current_probe) {
        selector = 0;
        animation_selector_request_probe = 0;
    } else if (selector != 0u) {
        animation_selector_current_probe = selector;
    }

    if ((scene_dispatch_blocked_probe & 1u) == 0u
            && (presentation_request_flags_probe & 2u) != 0u) {
        frame_delay_probe = 2u;
        return;
    }
    if (frame_delay_probe != 0u) {
        --frame_delay_probe;
        return;
    }

    api_request_probe.cursor.x = mouse_x_probe;
    api_request_probe.cursor.y = mouse_y_probe;
    api_request_probe.animation_selector = selector;
    api_request_probe.framebuffer_window_offset =
            (u16)graphics_draw_page_offset_probe;
    overlay_entry_probe(&api_request_probe);
}
