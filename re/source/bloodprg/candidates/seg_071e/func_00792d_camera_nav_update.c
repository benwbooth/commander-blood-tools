#include <dos.h>
#include <string.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define CAMERA_NAV_OBJECT_AT(type, offset) \
    ((volatile type CB_FAR *)MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define CAMERA_NAV_OBJECT_AT(type, offset) \
    ((volatile type CB_FAR *)(vm_record_base + (offset)))
#endif

#define CAMERA_NAV_OBJECT_KIND_MASK 0x0018u
#define CAMERA_NAV_REGION_RESULT 31
#define CAMERA_NAV_SLOT_INDEX 3u
#define CAMERA_NAV_SLOT_LOCK_FLAG 0x02u
#define CAMERA_NAV_SLOT_READY_FLAG 0x08u

typedef struct bloodprg_camera_nav_arche {
    cb_u8 reserved_00[0x16];
    cb_u16 current_location_offset;
} bloodprg_camera_nav_arche;

typedef struct bloodprg_camera_nav_location {
    cb_u16 kind;
    cb_u8 reserved_02[0x12];
    cb_u16 access_count;
} bloodprg_camera_nav_location;

void CB_NEAR camera_nav_update(void)
{
    volatile bloodprg_camera_nav_arche CB_FAR *arche;
    volatile bloodprg_camera_nav_location CB_FAR *location;
    cb_u8 slot_flags;

    if ((nav_camera_view_active & 1u) != 0u
            || nav_camera_approach_phase != 0u) {
        return;
    }

    arche = CAMERA_NAV_OBJECT_AT(
            bloodprg_camera_nav_arche, vm_arche_record_offset);
    location = CAMERA_NAV_OBJECT_AT(
            bloodprg_camera_nav_location, arche->current_location_offset);
    if ((location->kind & CAMERA_NAV_OBJECT_KIND_MASK) == 0u) {
        return;
    }
    if (ui_region_31_poll() != CAMERA_NAV_REGION_RESULT) {
        return;
    }

    nav_actor_presentation_state = 12u;
    if (location->access_count == 0u) {
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
        slot_flags = nav_actor_slots[CAMERA_NAV_SLOT_INDEX].flags;
        if ((slot_flags & CAMERA_NAV_SLOT_LOCK_FLAG) == 0u) {
            nav_actor_slots[CAMERA_NAV_SLOT_INDEX].flags =
                    slot_flags | CAMERA_NAV_SLOT_READY_FLAG;
        }
        return;
    }

    _fmemset((void CB_FAR *)palette_transition_source_gs, 0, 768u);
    _fmemcpy((void CB_FAR *)palette_transition_target,
            (const void CB_FAR *)live_palette,
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
