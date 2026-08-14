#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_4_UI_GATE 0x20u

void CB_NEAR nav_actor_handler_4(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    cb_u8 flags;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_4_UI_GATE) == 0u) {
        return;
    }

    line->flags |= NAV_ACTOR_PRESENT_FLAG;
    flags = line->flags;
    if ((flags & NAV_ACTOR_LOADED_FLAG) == 0u) {
        if ((flags & NAV_ACTOR_READY_FLAG) == 0u) {
            return;
        }
        if (nav_deferred_record_link == 0u
                && nav_pending_record_link == 0u) {
            line->flags = NAV_ACTOR_PRESENT_FLAG;
            return;
        }
    }

    nav_actor_presentation_state = 4u;
    if (!presentation_line_helper(line)) {
        return;
    }

    snd_play_clip(2);
    nav_deferred_record_link = nav_pending_record_link;
    nav_deferred_record_type = 0x00c4u;
    nav_pending_record_link = 0u;
    line->flags = NAV_ACTOR_PRESENT_FLAG;
    entity_flag_state_transition(4u);
    vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    snd_bank_loader(1u, nav_radio_snd_path);
}
