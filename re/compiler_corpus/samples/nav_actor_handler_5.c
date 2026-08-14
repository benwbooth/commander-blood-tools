typedef unsigned char u8;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_TRANSITION_FLAG 0x02u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_5_UI_GATE 0x10u
#define PRESENTATION_UI_REDRAW_FLAG 0x04u
#define NAV_CAMERA_VIEW_ACTIVE_FLAG 0x01u

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
extern volatile u8 nav_camera_view_active;
extern volatile u8 nav_camera_view_state;
extern volatile u8 nav_location_panel_active;
extern volatile u8 nav_actor_5_active;
extern volatile u16 nav_selected_location_record;
extern volatile u8 nav_screen_rebuild_pending;
extern volatile u8 nav_actor_0_busy;
extern volatile u8 nav_actor_1_busy;
extern volatile u16 nav_actor_presentation_state;
extern volatile u8 mouse_primary_pressed;

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
u16 FAR page_flip(void);
void FAR snd_play_clip(int clip_index);
void FAR entity_flag_state_transition(u16 object_id);
void FAR ship_3d_hud_palette_snapshot_and_camera_reset(void);

#pragma aux page_flip value [ax] modify exact [ax bx]
#pragma aux snd_play_clip parm [ax] modify exact []
#pragma aux entity_flag_state_transition parm [ax]
#pragma aux ship_3d_hud_palette_snapshot_and_camera_reset modify exact [bx dx]

void NEAR nav_actor_handler_5(volatile presentation_line_record NEAR *line)
{
    u8 transition_flags;
    int line_completed;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_5_UI_GATE) == 0u) {
        return;
    }

    if ((nav_actor_5_active & 1u) == 0u) {
        line->flags |= NAV_ACTOR_PRESENT_FLAG;
        transition_flags = line->flags;
        if ((transition_flags & NAV_ACTOR_READY_FLAG) == 0u) {
            goto transition_test;
        }
    }

    if ((nav_actor_1_busy | nav_actor_0_busy) != 0u) {
        nav_actor_5_active = 1u;
        nav_location_panel_active = 0u;
        return;
    }

    entity_flag_state_transition(0u);
    nav_selected_location_record = 0u;
    nav_actor_presentation_state = 10u;
    mouse_primary_pressed = 0u;
    transition_flags = 0u;
    line_completed = presentation_line_helper(line);
    if (line->frame_index == 7u) {
        if ((nav_camera_view_active & NAV_CAMERA_VIEW_ACTIVE_FLAG) == 0u) {
            transition_flags = (u8)page_flip();
        }
        snd_play_clip(3);
        nav_camera_view_state = 8u;
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
    }
    if (line_completed) {
        nav_actor_5_active = 0u;
        transition_flags = 7u;
        line->flags = transition_flags;
    }

transition_test:
    if ((transition_flags & NAV_ACTOR_TRANSITION_FLAG) == 0u) {
        return;
    }

    nav_camera_view_active ^= NAV_CAMERA_VIEW_ACTIVE_FLAG;
    vm_ui_flags &= (u8)~PRESENTATION_UI_REDRAW_FLAG;
    if ((nav_camera_view_active & NAV_CAMERA_VIEW_ACTIVE_FLAG) != 0u) {
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
    } else {
        ship_3d_hud_palette_snapshot_and_camera_reset();
        nav_screen_rebuild_pending = 1u;
    }
    nav_location_panel_active = 0u;
    line->flags = NAV_ACTOR_PRESENT_FLAG;
    entity_flag_state_transition(4u);
}
