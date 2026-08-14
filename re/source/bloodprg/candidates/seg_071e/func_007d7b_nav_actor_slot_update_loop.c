#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_ship3d.h"

#define NAV_ACTOR_SLOT_COUNT 6u
#define NAV_ACTOR_ACTIVE_FLAG 0x01u
#define NAV_ACTOR_LOCK_FLAG 0x02u
#define NAV_ACTOR_CLEAR_MOUSE_FLAG 0x04u
#define NAV_ACTOR_AUTO_SEEK_FLAG 0x08u
#define NAV_ACTOR_SEEK_UI_FLAG 0x08u

void CB_NEAR nav_actor_slot_update_loop(void)
{
    volatile bloodprg_nav_actor_slot CB_NEAR *slot;
    cb_u16 index;
    cb_u16 current_arc;
    cb_u8 busy;
    cb_u8 flags;

    busy = vm_presentation_active;
    busy |= vm_c2_presentation_gate;
    busy |= nav_choice_phase;
    busy |= save_request_active;
    busy |= load_request_active;
    busy |= (cb_u8)nav_console_selected_item;
    busy |= nav_target_selection;
    busy |= nav_transition_pending;
    busy |= ship_3d_nav_choice_sound_gate;
    if (busy != 0u) {
        return;
    }

    slot = nav_actor_slots;
    for (index = 0u; index < NAV_ACTOR_SLOT_COUNT; ++index, ++slot) {
        flags = slot->flags;
        if ((flags & NAV_ACTOR_ACTIVE_FLAG) != 0u) {
            if ((flags & NAV_ACTOR_CLEAR_MOUSE_FLAG) != 0u) {
                mouse_primary_pressed = 0u;
                mouse_press_pending = 0u;
            }

            mouse_hit_test(&slot->hit_rect, &slot->flags);
            flags = slot->flags;
            current_arc = (cb_u16)((cb_u16)vm_bridge_view_frame * 2u);
            if ((flags & NAV_ACTOR_AUTO_SEEK_FLAG) != 0u
                    && current_arc != slot->target_arc) {
                nav_bridge_seek_target_arc = slot->target_arc;
                vm_ui_flags |= NAV_ACTOR_SEEK_UI_FLAG;
            } else if ((flags & NAV_ACTOR_LOCK_FLAG) != 0u) {
                current_arc = (cb_u16)((cb_u16)vm_bridge_view_frame * 2u);
                if (current_arc != slot->target_arc) {
                    slot->flags = NAV_ACTOR_ACTIVE_FLAG;
                    entity_flag_state_transition(4u);
                }
            }
        }

        nav_actor_handlers[NAV_ACTOR_SLOT_COUNT - 1u - index](
                (volatile bloodprg_presentation_line_record CB_NEAR *)slot);
    }
}
