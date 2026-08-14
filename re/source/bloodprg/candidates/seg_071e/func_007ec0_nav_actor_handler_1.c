#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_nav.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define NAV_OBJECT_AT(type, offset) \
    ((volatile type CB_FAR *)MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define NAV_OBJECT_AT(type, offset) \
    ((volatile type CB_FAR *)(vm_record_base + (offset)))
#endif

#define NAV_ACTOR_PRESENT_FLAG 0x01u
#define NAV_ACTOR_LOADED_FLAG 0x04u
#define NAV_ACTOR_READY_FLAG 0x08u
#define NAV_ACTOR_HANDLER_1_UI_GATE 0x10u
#define NAV_KIND100 0x0100u

typedef struct bloodprg_nav_arche_object {
    cb_u8 reserved_00[0x16];
    cb_u16 current_location_offset;
} bloodprg_nav_arche_object;

void CB_NEAR nav_actor_handler_1(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    volatile bloodprg_nav_arche_object CB_FAR *arche;
    volatile bloodprg_vm_object_header CB_FAR *target;
    cb_u16 target_offset;
    cb_u8 flags;

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
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
    } else {
        arche = NAV_OBJECT_AT(
                bloodprg_nav_arche_object, vm_arche_record_offset);
        target_offset = arche->current_location_offset;
        target = NAV_OBJECT_AT(bloodprg_vm_object_header, target_offset);
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
        vm_ui_flags |= BLOODPRG_PRESENTATION_UI_REDRAW_FLAG;
        line->resource_id = 0x13u;
    }
}
