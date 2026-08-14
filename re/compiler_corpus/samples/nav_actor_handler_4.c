typedef unsigned char u8;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_4_UI_GATE 0x20u
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
extern volatile u16 nav_actor_presentation_state;
extern volatile u16 nav_pending_record_link;
extern volatile u16 nav_deferred_record_type;
extern volatile u16 nav_deferred_record_link;
extern volatile char nav_radio_snd_path[];

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
void FAR snd_play_clip(int clip_index);
void FAR entity_flag_state_transition(u16 object_id);
void FAR snd_bank_loader(u16 mode, volatile char NEAR *path);

#pragma aux snd_play_clip parm [ax] modify exact []
#pragma aux entity_flag_state_transition parm [ax]
#pragma aux snd_bank_loader parm [ax] [si] modify exact []

void NEAR nav_actor_handler_4(volatile presentation_line_record NEAR *line)
{
    u8 flags;

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
    vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
    snd_bank_loader(1u, nav_radio_snd_path);
}
