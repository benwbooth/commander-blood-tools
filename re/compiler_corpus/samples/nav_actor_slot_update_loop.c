typedef unsigned char u8;
typedef signed short i16;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far
#define CODE_DATA __based(__segname("_CODE"))

#define NAV_ACTOR_SLOT_COUNT 6u
#define NAV_ACTOR_ACTIVE_FLAG 0x01u
#define NAV_ACTOR_LOCK_FLAG 0x02u
#define NAV_ACTOR_CLEAR_MOUSE_FLAG 0x04u
#define NAV_ACTOR_AUTO_SEEK_FLAG 0x08u
#define NAV_ACTOR_SEEK_UI_FLAG 0x08u

typedef struct rect_i16 {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16;

typedef struct presentation_line_record {
    u8 bytes[24];
} presentation_line_record;

typedef struct nav_actor_slot {
    u8 flags;
    u8 reserved_01[9];
    u16 target_arc;
    rect_i16 hit_rect;
    u8 reserved_14[4];
} nav_actor_slot;

typedef void (NEAR *nav_actor_handler)(
        volatile presentation_line_record NEAR *line);

extern volatile u8 vm_presentation_active;
extern volatile u8 vm_c2_presentation_gate;
extern volatile u8 nav_choice_phase;
extern volatile u8 save_request_active;
extern volatile u8 load_request_active;
extern volatile u16 nav_console_selected_item;
extern volatile u8 nav_target_selection;
extern volatile u8 nav_transition_pending;
extern volatile u8 ship_3d_nav_choice_sound_gate;
extern volatile u8 vm_ui_flags;
extern volatile i16 vm_bridge_view_frame;
extern volatile u16 nav_bridge_seek_target_arc;
extern volatile u8 mouse_primary_pressed;
extern volatile u8 mouse_press_pending;
extern volatile nav_actor_slot nav_actor_slots[6];
extern nav_actor_handler CODE_DATA nav_actor_handlers[6];

void NEAR mouse_hit_test(
        const volatile rect_i16 NEAR *rect, volatile u8 NEAR *flags);
void FAR entity_flag_state_transition(u16 object_id);

#pragma aux entity_flag_state_transition parm [ax]

void NEAR nav_actor_slot_update_loop(void)
{
    volatile nav_actor_slot NEAR *slot;
    u16 index;
    u16 current_arc;
    u8 busy;
    u8 flags;

    busy = vm_presentation_active;
    busy |= vm_c2_presentation_gate;
    busy |= nav_choice_phase;
    busy |= save_request_active;
    busy |= load_request_active;
    busy |= (u8)nav_console_selected_item;
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
            current_arc = (u16)((u16)vm_bridge_view_frame * 2u);
            if ((flags & NAV_ACTOR_AUTO_SEEK_FLAG) != 0u
                    && current_arc != slot->target_arc) {
                nav_bridge_seek_target_arc = slot->target_arc;
                vm_ui_flags |= NAV_ACTOR_SEEK_UI_FLAG;
            } else if ((flags & NAV_ACTOR_LOCK_FLAG) != 0u) {
                current_arc = (u16)((u16)vm_bridge_view_frame * 2u);
                if (current_arc != slot->target_arc) {
                    slot->flags = NAV_ACTOR_ACTIVE_FLAG;
                    entity_flag_state_transition(4u);
                }
            }
        }

        nav_actor_handlers[NAV_ACTOR_SLOT_COUNT - 1u - index](
                (volatile presentation_line_record NEAR *)slot);
    }
}
