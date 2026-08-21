#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define SHIP_3D_TARGET_NONE 0xffffu
#define SHIP_3D_TARGET_NAME_BYTES 4u
#define SHIP_3D_TARGET_OPEN_STEP 6u

cb_u16 CB_NEAR ship_3d_target_record_select(void)
{
    const volatile cb_u16 CB_NEAR *items;
    const volatile void CB_FAR *label_segment_anchor;
    cb_i16 selection;
    cb_u16 selected;
    int transition_complete;

    ship_3d_target_fallback = 0u;
    items = ship_3d_presentable_name_offsets;
    label_segment_anchor = vm_record_base;
    if (*items == SHIP_3D_TARGET_NONE) {
        items = ship_3d_fallback_target_table;
        label_segment_anchor = (const volatile void CB_FAR *)items;
        ship_3d_target_fallback = 1u;
    }

    if ((ship_3d_target_select_phase & 1u) != 0u) {
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified(
                (const cb_u16 CB_NEAR *)items, label_segment_anchor);
        presentation_list_editing = 0u;
        framebuffer_transition_current_step = 0u;
        ++ship_3d_target_select_phase;
    }

    if ((ship_3d_target_select_phase & 2u) != 0u) {
        transition_complete = framebuffer_transition_current_step
                == framebuffer_transition_total_steps;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                &ship_3d_target_transition_rect);
        if (!transition_complete) {
            return 0u;
        }
        ship_3d_target_select_phase = 0u;
    }

    selection = list_widget_layout_unified(
            (const cb_u16 CB_NEAR *)items, label_segment_anchor);
    if (selection == (cb_i16)SHIP_3D_TARGET_NONE) {
        return 0u;
    }

    selected = items[(cb_u16)selection];
    if (selected == SHIP_3D_TARGET_NONE) {
        ship_3d_depth_opening = 1u;
        ship_3d_depth_step = SHIP_3D_TARGET_OPEN_STEP;
        return SHIP_3D_TARGET_NONE;
    }

    selected = (cb_u16)(selected - SHIP_3D_TARGET_NAME_BYTES);
    if ((ship_3d_target_fallback & 1u) != 0u) {
        selected = ship_3d_current_target;
    }
    return selected;
}
