#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned long u32;

#define NEAR __near
#define FAR __far

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_2_UI_GATE 0x90u

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
extern volatile u16 vm_ship_active_flags;
extern volatile u16 nav_actor_ship_depth_offset;
extern u32 nav_actor_live_palette_dwords[0x90];
extern u32 nav_actor_bridge_palette_dwords[0x90];

int NEAR presentation_line_helper(volatile presentation_line_record NEAR *line);
void FAR snd_play_clip(int clip_index);

#pragma aux snd_play_clip parm [ax] modify exact []

void NEAR nav_actor_handler_2(volatile presentation_line_record NEAR *line)
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
