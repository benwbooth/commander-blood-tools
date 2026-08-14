typedef unsigned char u8;
typedef signed short i16;
typedef unsigned short u16;

#define NEAR __near

#define PRESENTATION_MODE_GATE 0x50u
#define PRESENTATION_MODE_SECOND_RECT 0x40u
#define PRESENTATION_MODE_ACTIVE_FLAG 0x01u

typedef struct rect_i16 {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16;

typedef struct nav_actor_slot {
    u8 flags;
    u8 reserved_01[9];
    u16 target_arc;
    rect_i16 hit_rect;
    u8 reserved_14[4];
} nav_actor_slot;

extern volatile u8 vm_ui_flags;
extern volatile i16 mouse_x;
extern volatile i16 mouse_y;
extern volatile u8 presentation_mode_active;
extern volatile u16 presentation_mode_previous_state;
extern volatile u16 nav_actor_presentation_state;
extern volatile nav_actor_slot nav_actor_slots[6];

void NEAR presentation_mode_dispatch(void)
{
    const volatile rect_i16 NEAR *rect;
    i16 point_x;
    i16 point_y;

    if ((vm_ui_flags & PRESENTATION_MODE_GATE) == 0u) {
        return;
    }

    if ((vm_ui_flags & PRESENTATION_MODE_SECOND_RECT) != 0u) {
        rect = &nav_actor_slots[2].hit_rect;
    } else {
        rect = &nav_actor_slots[0].hit_rect;
    }

    point_x = mouse_x;
    if (point_x < rect->x) {
        goto outside;
    }
    point_x = (i16)((u16)point_x - (u16)rect->width);
    if (point_x > rect->x) {
        goto outside;
    }

    point_y = mouse_y;
    if (point_y < rect->y) {
        goto outside;
    }
    point_y = (i16)((u16)point_y - (u16)rect->height);
    if (point_y > rect->y) {
        goto outside;
    }

    if ((presentation_mode_active & PRESENTATION_MODE_ACTIVE_FLAG) == 0u) {
        presentation_mode_active = PRESENTATION_MODE_ACTIVE_FLAG;
        nav_actor_presentation_state = 9u;
    }
    return;

outside:
    if ((presentation_mode_active & PRESENTATION_MODE_ACTIVE_FLAG) != 0u) {
        presentation_mode_active = 0u;
        nav_actor_presentation_state = presentation_mode_previous_state;
    }
}
