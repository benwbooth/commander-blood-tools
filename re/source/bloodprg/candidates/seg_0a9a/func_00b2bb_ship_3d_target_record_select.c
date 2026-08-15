#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define SHIP_3D_TARGET_NONE 0xffffu
#define SHIP_3D_TARGET_NAME_BYTES 4u
#define SHIP_3D_TARGET_OPEN_STEP 6u

#if defined(__TURBOC__) || defined(__BORLANDC__)
#pragma warn -rch
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define SHIP_3D_TARGET_SEGMENT(pointer) \
    FP_SEG((const void CB_FAR *)(pointer))
#else
#define SHIP_3D_TARGET_SEGMENT(pointer) 0u
#endif

#if defined(__WATCOMC__)
static cb_i16 CB_NEAR ship_3d_target_list_widget(
        const cb_u16 CB_NEAR *items,
        cb_u16 string_segment);
#pragma aux ship_3d_target_list_widget = \
        "mov es,dx" \
        "call far ptr list_widget_layout_unified" \
        parm [si] [dx] value [ax] modify exact [ax es]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
static cb_i16 CB_NEAR ship_3d_target_list_widget(
        const cb_u16 CB_NEAR *items,
        cb_u16 string_segment)
{
    cb_i16 result;

    if (0) {
        return list_widget_layout_unified(items);
    }
    /* Borland cannot declare the original SI/ES far-call ABI. */
    asm push si;
    asm push es;
    asm mov si, items;
    asm mov es, string_segment;
    asm call far ptr _list_widget_layout_unified;
    asm mov result, ax;
    asm pop es;
    asm pop si;
    return result;
}
#else
static cb_i16 CB_NEAR ship_3d_target_list_widget(
        const cb_u16 CB_NEAR *items,
        cb_u16 string_segment)
{
    (void)string_segment;
    return list_widget_layout_unified(items);
}
#endif

cb_u16 CB_NEAR ship_3d_target_record_select(void)
{
    const volatile cb_u16 CB_NEAR *items;
    cb_i16 selection;
    cb_u16 selected;
    cb_u16 string_segment;
    int transition_complete;

    ship_3d_target_fallback = 0u;
    items = ship_3d_presentable_name_offsets;
    string_segment = SHIP_3D_TARGET_SEGMENT(vm_record_base);
    if (*items == SHIP_3D_TARGET_NONE) {
        items = ship_3d_fallback_target_table;
        string_segment = SHIP_3D_TARGET_SEGMENT(items);
        ship_3d_target_fallback = 1u;
    }

    if ((ship_3d_target_select_phase & 1u) != 0u) {
        presentation_list_editing = 1u;
        (void)ship_3d_target_list_widget((const cb_u16 CB_NEAR *)items,
                string_segment);
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

    selection = ship_3d_target_list_widget((const cb_u16 CB_NEAR *)items,
            string_segment);
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

#undef SHIP_3D_TARGET_SEGMENT
