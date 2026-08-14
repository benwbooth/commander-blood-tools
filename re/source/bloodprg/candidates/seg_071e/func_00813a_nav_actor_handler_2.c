#include <string.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_2_UI_GATE 0x90u

void CB_NEAR nav_actor_handler_2(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    if ((vm_ui_flags & NAV_ACTOR_HANDLER_2_UI_GATE) == 0u) {
        return;
    }

    line->flags |= NAV_ACTOR_PRESENT_FLAG;
    if ((line->flags & NAV_ACTOR_READY_FLAG) == 0u) {
        return;
    }

    nav_actor_presentation_state = 0x10u;
    if (!presentation_line_helper(line)) {
        return;
    }

    snd_play_clip(5);
    vm_ship_active_flags = 1u;
    memcpy(nav_actor_bridge_palette_dwords,
            nav_actor_live_palette_dwords,
            sizeof(nav_actor_live_palette_dwords));
    nav_actor_ship_depth_offset = 0u;
    line->flags = 7u;
}
