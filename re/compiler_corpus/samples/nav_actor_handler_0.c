typedef unsigned char u8;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_0_UI_GATE 0x10u
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
extern volatile u16 nav_target_presentation_state;
extern volatile u16 nav_actor_presentation_state;
extern volatile u16 nav_deferred_record_type;
extern volatile u16 nav_deferred_record_link;
extern volatile u8 nav_transition_pending;
extern volatile u8 nav_location_panel_active;
extern volatile u8 nav_actor_0_busy;
extern volatile u8 nav_presentation_reverse;
extern volatile u8 nav_camera_view_state;

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
void FAR entity_flag_state_transition(u16 object_id);
void FAR snd_play_clip(int clip_index);

#pragma aux entity_flag_state_transition parm [ax]
#pragma aux snd_play_clip parm [ax] modify exact []

void NEAR nav_actor_handler_0(volatile presentation_line_record NEAR *line)
{
    u8 flags;
    u8 deferred_gate;
    u8 second_pass_prepared;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_0_UI_GATE) == 0u
            || nav_actor_0_busy != 0u) {
        return;
    }

    flags = line->flags;
    second_pass_prepared = (u8)((flags & NAV_ACTOR_LOADED_FLAG) != 0u);

    if ((flags & NAV_ACTOR_PRESENT_FLAG) != 0u) {
        if ((flags & NAV_ACTOR_READY_FLAG) != 0u) {
            nav_target_presentation_state = 0u;
            nav_actor_presentation_state = 10u;
            entity_flag_state_transition(0u);
            entity_flag_state_transition(4u);
            (void)presentation_line_helper(line);
            second_pass_prepared = 1u;

            if (line->frame_index == 1u) {
                nav_camera_view_state = 8u;
            } else if (nav_camera_view_state == 0u) {
                line->flags = 7u;
                nav_deferred_record_type = 0x00c1u;
                nav_transition_pending = 1u;
                entity_flag_state_transition(4u);
                nav_location_panel_active = 0u;
                vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
                return;
            }
        }

        if ((nav_location_panel_active & 1u) != 0u) {
            return;
        }
        line->resource_id = 0x14u;
        nav_presentation_reverse = 1u;
        line->flags = 0u;
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
    }

    if (nav_deferred_record_link == 0u) {
        return;
    }
    deferred_gate = nav_presentation_reverse;
    if ((deferred_gate |= nav_location_panel_active) == 0u) {
        return;
    }

    if (second_pass_prepared == 0u) {
        entity_flag_state_transition(4u);
        line->resource_id = 0x14u;
        snd_play_clip(5);
    }

    if (!presentation_line_helper(line)) {
        return;
    }
    if ((nav_location_panel_active & 1u) == 0u) {
        nav_deferred_record_link = 0u;
        entity_flag_state_transition(4u);
        line->flags = 0u;
    } else {
        line->flags = NAV_ACTOR_PRESENT_FLAG;
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
        line->resource_id = 0x12u;
    }
}
