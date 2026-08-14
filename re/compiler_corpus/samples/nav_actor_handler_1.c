#include <dos.h>

typedef unsigned char u8;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far
#define NAV_OBJECT_AT(type, offset) \
    ((volatile type FAR *)MK_FP(FP_SEG(vm_record_base), (offset)))

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_1_UI_GATE 0x10u
#define PRESENTATION_UI_REDRAW_FLAG 0x04u
#define NAV_KIND100 0x0100u

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

typedef struct vm_object_header {
    u16 kind;
    u8 flags;
} vm_object_header;

typedef struct nav_arche_object {
    u8 reserved_00[0x16];
    u16 current_location_offset;
} nav_arche_object;

extern volatile u8 vm_ui_flags;
extern volatile u8 FAR *vm_record_base;
extern volatile u16 vm_arche_record_offset;
extern volatile u16 nav_target_presentation_state;
extern volatile u16 nav_actor_presentation_state;
extern volatile u16 nav_deferred_record_type;
extern volatile u16 nav_deferred_record_link;
extern volatile u8 nav_actor_transition_phase;
extern volatile u16 nav_kind100_target_record;
extern volatile u8 nav_location_panel_active;
extern volatile u8 nav_actor_5_active;
extern volatile u8 nav_actor_1_busy;
extern volatile u8 nav_presentation_reverse;
extern volatile u8 nav_camera_view_active;

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
void FAR entity_flag_state_transition(u16 object_id);
void FAR snd_play_clip(int clip_index);

#pragma aux entity_flag_state_transition parm [ax]
#pragma aux snd_play_clip parm [ax] modify exact []

void NEAR nav_actor_handler_1(volatile presentation_line_record NEAR *line)
{
    volatile nav_arche_object FAR *arche;
    volatile vm_object_header FAR *target;
    u16 target_offset;
    u8 flags;

    if ((vm_ui_flags & NAV_ACTOR_HANDLER_1_UI_GATE) == 0u
            || nav_actor_1_busy != 0u) {
        return;
    }

    flags = line->flags;
    if ((flags & NAV_ACTOR_PRESENT_FLAG) != 0u) {
        if ((flags & NAV_ACTOR_READY_FLAG) != 0u) {
            nav_target_presentation_state = 0u;
            nav_actor_presentation_state = 11u;
            if (presentation_line_helper(line)) {
                nav_deferred_record_type = 0x00c6u;
                nav_deferred_record_link = nav_kind100_target_record;
                nav_actor_transition_phase = 0u;
                line->flags = 0u;
                goto retarget_line;
            }
        }
        if ((nav_location_panel_active & 1u) == 0u
                && (nav_actor_5_active & 1u) == 0u) {
            return;
        }

retarget_line:
        line->resource_id = 0x15u;
        nav_presentation_reverse = 1u;
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
    } else {
        arche = NAV_OBJECT_AT(nav_arche_object, vm_arche_record_offset);
        target_offset = arche->current_location_offset;
        target = NAV_OBJECT_AT(vm_object_header, target_offset);
        if (target->kind != NAV_KIND100) {
            return;
        }
        nav_kind100_target_record = target_offset;
        if ((nav_presentation_reverse | nav_camera_view_active) == 0u) {
            return;
        }
        if ((flags & NAV_ACTOR_LOADED_FLAG) == 0u) {
            if ((nav_actor_5_active & 1u) != 0u) {
                return;
            }
            entity_flag_state_transition(4u);
            line->resource_id = 0x15u;
            snd_play_clip(5);
        }
    }

    if (!presentation_line_helper(line)) {
        return;
    }
    if ((nav_actor_5_active & 1u) != 0u
            || (nav_location_panel_active & 1u) != 0u) {
        entity_flag_state_transition(4u);
        line->flags = 0u;
    } else {
        line->flags = NAV_ACTOR_PRESENT_FLAG;
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
        line->resource_id = 0x13u;
    }
}
