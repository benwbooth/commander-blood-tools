#include <dos.h>

#include "../include/bloodprg_input.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define BRIDGE_SEEK_FLAG 0x0008u
#define BRIDGE_MENU_ENGAGED_FLAG 0x0004u
#define BRIDGE_FRAME_COUNT 180u
#define BRIDGE_ARC_UNITS 360u
#define BRIDGE_RING_UNITS 1440u
#define BRIDGE_SEEK_DRAG_THRESHOLD 40
#define BRIDGE_STEER_DEAD_ZONE 31
#define BRIDGE_MENU_CLAMP_DISTANCE 40
#define BRIDGE_STEER_TRAIL 30u
#define BRIDGE_FRAME_RING_UNITS 8u
#define BRIDGE_HALF_SCREEN 160u

int CB_FAR bridge_steer_update(
        cb_u16 CB_NEAR *presentation_link_target)
{
    union REGS registers;
    cb_u16 frame;
    cb_u16 target_frame;
    cb_u16 mouse_ring;
    cb_u16 view_arc;
    cb_u16 mouse_arc;
    cb_u16 distance;
    cb_u16 positive_arc;
    cb_u16 step;
    cb_u16 drag;
    cb_u16 link_target;
    int view_changed;

    frame = (cb_u16)vm_bridge_view_frame;
    mouse_ring = (cb_u16)mouse_x;
    link_target = presentation_link_target != 0
            ? *presentation_link_target
            : 0u;
    view_changed = 0;

    if ((vm_ui_state.word & BRIDGE_SEEK_FLAG) != 0u) {
        target_frame = (cb_u16)(nav_bridge_seek_target_arc >> 1);
        if (frame == target_frame) {
            vm_ui_state.word ^= BRIDGE_SEEK_FLAG;
            bridge_seek_initial_distance = 0u;
        } else {
            if ((cb_i16)frame > (cb_i16)target_frame) {
                distance = (cb_u16)(frame - target_frame);
            } else {
                distance = (cb_u16)(target_frame - frame);
            }
            if ((cb_i16)distance >= (cb_i16)(BRIDGE_FRAME_COUNT / 2u)) {
                distance = (cb_u16)(BRIDGE_FRAME_COUNT - distance);
            }

            positive_arc = (cb_u16)(frame + distance);
            if ((cb_i16)positive_arc < 0) {
                positive_arc = (cb_u16)(positive_arc + BRIDGE_FRAME_COUNT);
            } else if ((cb_i16)positive_arc >= (cb_i16)BRIDGE_FRAME_COUNT) {
                positive_arc = (cb_u16)(positive_arc - BRIDGE_FRAME_COUNT);
            }
            positive_arc = (cb_u16)(positive_arc << 1);

            if (bridge_seek_initial_distance == 0u) {
                bridge_seek_initial_distance = distance;
            }
            step = (cb_u16)(distance >> 1);
            if (step == 0u) {
                step = 1u;
            }
            drag = (cb_u16)(distance << 2);
            bridge_turn_direction = 1u;
            if (positive_arc != nav_bridge_seek_target_arc) {
                bridge_turn_direction = 0u;
                step = (cb_u16)(0u - step);
                drag = (cb_u16)(0u - drag);
            }

            if ((cb_i16)bridge_seek_initial_distance
                    >= BRIDGE_SEEK_DRAG_THRESHOLD) {
                mouse_ring = (cb_u16)(mouse_ring + drag);
                mouse_last_x = (cb_i16)((cb_u16)mouse_last_x + drag);
            }

            frame = (cb_u16)(frame + step);
            if ((cb_i16)frame < 0) {
                frame = (cb_u16)(frame + BRIDGE_FRAME_COUNT);
            } else if ((cb_i16)frame >= (cb_i16)BRIDGE_FRAME_COUNT) {
                frame = (cb_u16)(frame - BRIDGE_FRAME_COUNT);
            }
            vm_bridge_view_frame = (cb_i16)frame;
            mouse_button_state = 0u;
        }
    }

    frame = (cb_u16)vm_bridge_view_frame;
    view_arc = (cb_u16)(frame << 1);
    mouse_ring = (cb_u16)(mouse_ring - BRIDGE_RING_UNITS);
    if ((cb_i16)mouse_ring < 0) {
        mouse_ring = (cb_u16)(mouse_ring + BRIDGE_RING_UNITS);
    } else if ((cb_i16)mouse_ring >= (cb_i16)BRIDGE_RING_UNITS) {
        mouse_ring = (cb_u16)(mouse_ring - BRIDGE_RING_UNITS);
    }

    registers.x.ax = 4u;
    registers.x.cx = (cb_u16)(mouse_ring + BRIDGE_RING_UNITS);
    registers.x.dx = (cb_u16)mouse_y;
    int86(0x33, &registers, &registers);

    mouse_x = (cb_i16)mouse_ring;
    mouse_arc = (cb_u16)(mouse_ring >> 2);
    bridge_mouse_arc = mouse_arc;

    if ((vm_ui_state.word & BRIDGE_SEEK_FLAG) != 0u) {
        view_changed = 1;
    } else {
        link_target = view_arc;
        if ((cb_i16)view_arc > (cb_i16)mouse_arc) {
            distance = (cb_u16)(view_arc - mouse_arc);
        } else {
            distance = (cb_u16)(mouse_arc - view_arc);
        }
        if ((cb_i16)distance >= (cb_i16)(BRIDGE_ARC_UNITS / 2u)) {
            distance = (cb_u16)(BRIDGE_ARC_UNITS - distance);
        }

        if ((cb_i16)distance > BRIDGE_STEER_DEAD_ZONE) {
            if ((vm_ui_state.word & BRIDGE_MENU_ENGAGED_FLAG) != 0u) {
                if ((cb_i16)distance >= BRIDGE_MENU_CLAMP_DISTANCE) {
                    positive_arc = (cb_u16)(mouse_arc + distance);
                    if ((cb_i16)positive_arc < 0) {
                        positive_arc =
                                (cb_u16)(positive_arc + BRIDGE_ARC_UNITS);
                    } else if ((cb_i16)positive_arc
                            >= (cb_i16)BRIDGE_ARC_UNITS) {
                        positive_arc =
                                (cb_u16)(positive_arc - BRIDGE_ARC_UNITS);
                    }

                    if (positive_arc == view_arc) {
                        link_target = (cb_u16)(view_arc
                                - BRIDGE_MENU_CLAMP_DISTANCE);
                    } else {
                        link_target = (cb_u16)(view_arc
                                + BRIDGE_MENU_CLAMP_DISTANCE);
                    }
                    if ((cb_i16)link_target < 0) {
                        link_target =
                                (cb_u16)(link_target + BRIDGE_ARC_UNITS);
                    } else if ((cb_i16)link_target
                            >= (cb_i16)BRIDGE_ARC_UNITS) {
                        link_target =
                                (cb_u16)(link_target - BRIDGE_ARC_UNITS);
                    }
                    link_target = (cb_u16)(link_target << 2);
                    mouse_ring = link_target;
                    mouse_x = (cb_i16)mouse_ring;

                    registers.x.ax = 4u;
                    registers.x.cx =
                            (cb_u16)(mouse_ring + BRIDGE_RING_UNITS);
                    registers.x.dx = (cb_u16)mouse_y;
                    int86(0x33, &registers, &registers);
                }
            } else {
                positive_arc = (cb_u16)(mouse_arc + distance);
                if ((cb_i16)positive_arc < 0) {
                    positive_arc =
                            (cb_u16)(positive_arc + BRIDGE_ARC_UNITS);
                } else if ((cb_i16)positive_arc
                        >= (cb_i16)BRIDGE_ARC_UNITS) {
                    positive_arc =
                            (cb_u16)(positive_arc - BRIDGE_ARC_UNITS);
                }

                if (positive_arc == view_arc) {
                    bridge_turn_direction = 0u;
                    mouse_arc = (cb_u16)(mouse_arc + BRIDGE_STEER_TRAIL);
                } else {
                    bridge_turn_direction = 1u;
                    mouse_arc = (cb_u16)(mouse_arc - BRIDGE_STEER_TRAIL);
                }
                if ((cb_i16)mouse_arc < 0) {
                    mouse_arc = (cb_u16)(mouse_arc + BRIDGE_ARC_UNITS);
                } else if ((cb_i16)mouse_arc >= (cb_i16)BRIDGE_ARC_UNITS) {
                    mouse_arc = (cb_u16)(mouse_arc - BRIDGE_ARC_UNITS);
                }
                frame = (cb_u16)(mouse_arc >> 1);
                vm_bridge_view_frame = (cb_i16)frame;
                view_changed = 1;
            }
        }
    }

    if (view_changed) {
        frame = (cb_u16)vm_bridge_view_frame;
        ship_3d_projection_angle_b = frame;
        bridge_frame_angle_bias =
                (cb_u16)(frame * BRIDGE_FRAME_RING_UNITS
                - BRIDGE_HALF_SCREEN);
        mouse_ring &= 0xFFF8u;
        mouse_x = (cb_i16)mouse_ring;
    }

    mouse_ring = (cb_u16)((cb_u16)mouse_x - bridge_frame_angle_bias);
    if ((cb_i16)mouse_ring < 0) {
        mouse_ring = (cb_u16)(mouse_ring + BRIDGE_RING_UNITS);
    } else if ((cb_i16)mouse_ring >= (cb_i16)BRIDGE_RING_UNITS) {
        mouse_ring = (cb_u16)(mouse_ring - BRIDGE_RING_UNITS);
    }
    mouse_x = (cb_i16)mouse_ring;

    if (presentation_link_target != 0) {
        *presentation_link_target = link_target;
    }
    return view_changed;
}
