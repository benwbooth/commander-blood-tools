#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define MANU3_MODE_ACTIVE 0x01u
#define MANU3_PRESENTATION_DELAY_TRIGGER 0x02u
#define MANU3_PRESENTATION_DELAY_FRAMES 2u

void CB_NEAR manu3_hand_frame_dispatch(void)
{
    cb_u16 selector;

    if ((presentation_mode_flag_27e0 & MANU3_MODE_ACTIVE) != 0u
            || (main_loop_hud_refresh_enabled & MANU3_MODE_ACTIVE) != 0u) {
        return;
    }

    selector = manu3_animation_selector_request;
    if ((cb_i16)selector < 0) {
        return;
    }

    if (selector == manu3_animation_selector_current) {
        selector = 0;
        manu3_animation_selector_request = 0;
    } else if (selector != 0u) {
        manu3_animation_selector_current = selector;
    }

    if ((ship_3d_scene_dispatch_blocked & MANU3_MODE_ACTIVE) == 0u
            && (vm_presentation_request_flags
                & MANU3_PRESENTATION_DELAY_TRIGGER) != 0u) {
        manu3_frame_delay = MANU3_PRESENTATION_DELAY_FRAMES;
        return;
    }
    if (manu3_frame_delay != 0u) {
        --manu3_frame_delay;
        return;
    }

    manu3_api_request.cursor.x = mouse_x;
    manu3_api_request.cursor.y = mouse_y;
    manu3_api_request.animation_selector = selector;
    manu3_api_request.framebuffer_window_offset =
            (cb_u16)graphics_draw_page_offset;
#if defined(BLOODPRG_RELINKED_RUNTIME)
    cb_overlay_call_inherited_bp(
            (bloodprg_overlay_entry_raw)manu3_overlay_entry,
            &manu3_api_request);
#else
    manu3_overlay_entry(&manu3_api_request);
#endif
}
