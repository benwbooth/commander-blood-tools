typedef unsigned char u8;
typedef signed short i16;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_3_UI_GATE 0x40u
#define PRESENTATION_UI_REDRAW_FLAG 0x04u

typedef struct presentation_line_record {
    u8 flags;
    u8 pad_01;
    u16 resource_id;
    u16 pad_04;
    u16 terminal_frame;
    u16 frame_index;
    u8 pad_0a[10];
    u16 draw_x;
    u16 draw_y;
} presentation_line_record;

extern volatile u8 vm_ui_flags;
extern volatile u8 presentation_mode_flag_27e1;
extern volatile u8 vm_c2_presentation_gate;
extern volatile u8 mouse_primary_pressed;
extern volatile u8 mouse_press_pending;
extern volatile u8 nav_actor_completion_latch;
extern volatile u16 nav_actor_presentation_state;
extern volatile i16 nav_actor_zoom_counter;

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
void FAR presentation_update_1fb2(void);
void FAR entity_flag_state_transition(u16 object_id);

#pragma aux entity_flag_state_transition parm [ax]

void NEAR nav_actor_handler_3(volatile presentation_line_record NEAR *line)
{
    if ((vm_ui_flags & NAV_ACTOR_HANDLER_3_UI_GATE) == 0u) {
        return;
    }

    line->flags |= NAV_ACTOR_PRESENT_FLAG;
    if ((line->flags & NAV_ACTOR_READY_FLAG) != 0u) {
        nav_actor_presentation_state = 13u;
        if ((presentation_mode_flag_27e1 & 1u) != 0u
                && nav_actor_zoom_counter < 100) {
            nav_actor_zoom_counter = 106;
            if ((vm_c2_presentation_gate & 1u) != 0u) {
                presentation_update_1fb2();
            }
        }

        mouse_primary_pressed = 0u;
        mouse_press_pending = 0u;
        if (presentation_line_helper(line)) {
            entity_flag_state_transition(4u);
            line->flags = NAV_ACTOR_PRESENT_FLAG;
            if ((presentation_mode_flag_27e1 & 1u) == 0u) {
                presentation_mode_flag_27e1 = 1u;
                vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
            }
        }
    }

    if ((presentation_mode_flag_27e1 & 1u) != 0u
            && (line->flags & NAV_ACTOR_LOADED_FLAG) != 0u) {
        nav_actor_completion_latch = 1u;
    }
}
