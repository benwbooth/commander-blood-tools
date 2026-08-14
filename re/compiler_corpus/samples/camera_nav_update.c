#include <dos.h>
#include <string.h>
typedef unsigned char u8;
typedef unsigned short u16;

#define NEAR __near
#define FAR __far
#define GAME_DATA __based(__segname("GAME_DATA"))
#define CAMERA_NAV_OBJECT_AT(type, offset) \
    ((volatile type FAR *)MK_FP(FP_SEG(vm_record_base), (offset)))

#define CAMERA_NAV_OBJECT_KIND_MASK 0x0018u
#define CAMERA_NAV_REGION_RESULT 31
#define CAMERA_NAV_SLOT_INDEX 3u
#define CAMERA_NAV_SLOT_LOCK_FLAG 0x02u
#define CAMERA_NAV_SLOT_READY_FLAG 0x08u
#define PRESENTATION_UI_REDRAW_FLAG 0x04u

typedef union vm_ui_state_type {
    u16 word;
    struct {
        u8 flags;
        u8 auxiliary;
    } bytes;
} vm_ui_state_type;

#define vm_ui_flags (vm_ui_state.bytes.flags)

typedef struct rect_i16 {
    short x;
    short y;
    short width;
    short height;
} rect_i16;

typedef struct nav_actor_slot {
    u8 flags;
    u8 reserved_01[9];
    u16 target_arc;
    rect_i16 hit_rect;
    u8 reserved_14[4];
} nav_actor_slot;

typedef struct vm_object_record {
    u16 kind;
    u8 reserved_02[0x12];
    u16 access_count;
} vm_object_record;

typedef struct camera_nav_arche {
    u8 reserved_00[0x16];
    u16 current_location_offset;
} camera_nav_arche;

extern volatile u8 nav_camera_view_active;
extern volatile u8 nav_camera_approach_phase;
extern volatile u8 FAR *vm_record_base;
extern volatile u16 vm_arche_record_offset;
extern volatile u16 nav_actor_presentation_state;
extern volatile nav_actor_slot nav_actor_slots[6];
extern volatile vm_ui_state_type vm_ui_state;
extern volatile u8 live_palette[768];
extern u8 GAME_DATA palette_transition_source_gs[768];
extern u8 GAME_DATA palette_transition_target[768];
extern volatile u16 palette_transition_percent;
extern volatile u16 palette_transition_increment;
extern volatile u8 palette_transition_first;
extern volatile u8 palette_transition_last;
extern volatile u16 vm_ship_active_flags;
extern volatile u8 ship_3d_hud_init_pending;
extern volatile u8 vm_dialogue_hold_complete;
extern volatile u8 ship_3d_scene_dispatch_blocked;
extern volatile u16 nav_actor_ship_depth_offset;
extern volatile u8 ship_3d_depth_opening;
extern volatile u8 ship_3d_hud_initialized;

short FAR ui_region_31_poll(void);

#pragma aux ui_region_31_poll value [ax] modify exact [ax]

void NEAR camera_nav_update(void)
{
    volatile camera_nav_arche FAR *arche;
    volatile vm_object_record FAR *location;
    u8 slot_flags;

    if ((nav_camera_view_active & 1u) != 0u
            || nav_camera_approach_phase != 0u) {
        return;
    }

    arche = CAMERA_NAV_OBJECT_AT(camera_nav_arche, vm_arche_record_offset);
    location = CAMERA_NAV_OBJECT_AT(vm_object_record, arche->current_location_offset);
    if ((location->kind & CAMERA_NAV_OBJECT_KIND_MASK) == 0u) {
        return;
    }
    if (ui_region_31_poll() != CAMERA_NAV_REGION_RESULT) {
        return;
    }

    nav_actor_presentation_state = 12u;
    if (location->access_count == 0u) {
        vm_ui_flags |= PRESENTATION_UI_REDRAW_FLAG;
        slot_flags = nav_actor_slots[CAMERA_NAV_SLOT_INDEX].flags;
        if ((slot_flags & CAMERA_NAV_SLOT_LOCK_FLAG) == 0u) {
            nav_actor_slots[CAMERA_NAV_SLOT_INDEX].flags =
                    slot_flags | CAMERA_NAV_SLOT_READY_FLAG;
        }
        return;
    }

    _fmemset((void FAR *)palette_transition_source_gs, 0, 768u);
    _fmemcpy((void FAR *)palette_transition_target,
            (const void FAR *)live_palette,
            768u);
    palette_transition_percent = 0u;
    palette_transition_increment = 0x14u;
    palette_transition_first = 0u;
    palette_transition_last = 0xffu;
    vm_ui_state.word = 0u;
    vm_ship_active_flags = 5u;
    ship_3d_hud_init_pending = 1u;
    vm_dialogue_hold_complete = 0u;
    ship_3d_scene_dispatch_blocked = 0u;
    nav_actor_ship_depth_offset = 0u;
    ship_3d_depth_opening = 0u;
    ship_3d_hud_initialized = 0u;
}
