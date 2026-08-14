#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define CAMERA_APPROACH_PHASE_MASK 0x07u
#define CAMERA_APPROACH_UI_FLAG 0x0004u
#define CAMERA_APPROACH_ENTITY_FIRST 21u
#define CAMERA_APPROACH_ENTITY_LAST 31u
#define CAMERA_APPROACH_X_LIMIT 9000
#define CAMERA_APPROACH_X_STEP 100u
#define CAMERA_APPROACH_ANGLE_WRAP 180u
#define CAMERA_APPROACH_Z_CRUISE 20000u
#define CAMERA_APPROACH_Z_ACCELERATION_STEP 100u
#define CAMERA_APPROACH_Z_FINAL 30000u
#define CAMERA_APPROACH_X_RESET 10000
#define CAMERA_APPROACH_HYPERSPACE_LINE 6u
#define CAMERA_APPROACH_SEQUENCE_MASK 0x0007u

void CB_NEAR camera_fsm_state_gate(cb_u16 scene_link_target)
{
    const char CB_NEAR *source;
    volatile char CB_GAME_DATA *destination;
    cb_u16 camera_word;
    cb_u16 angle;
    cb_u16 easing_step;
    cb_u8 character;
    cb_u8 phase;

    if ((nav_camera_approach_phase & CAMERA_APPROACH_PHASE_MASK) == 0u) {
        nav_actor_presentation_state = 1u;
        nav_camera_view_active = 0u;
        nav_transition_pending = 1u;
        screen_flags_init();
        ++nav_camera_approach_phase;
        vm_ui_state.word |= CAMERA_APPROACH_UI_FLAG;
    }

    phase = nav_camera_approach_phase;
    if (phase == 1u) {
        camera_word = (cb_u16)ship_3d_camera_x;
        if ((cb_i16)camera_word >= CAMERA_APPROACH_X_LIMIT) {
            ship_3d_camera_x = (cb_i16)(camera_word - CAMERA_APPROACH_X_STEP);
            angle = (cb_u16)(ship_3d_projection_angle_a - 1u);
            if ((cb_i16)angle < 0) {
                angle = CAMERA_APPROACH_ANGLE_WRAP;
            }
            ship_3d_projection_angle_a = angle;
        } else {
            ++nav_camera_approach_phase;
        }
    } else if (phase == 2u) {
        camera_word = (cb_u16)ship_3d_camera_z;
        if (camera_word <= CAMERA_APPROACH_Z_CRUISE) {
            ship_3d_camera_z = (cb_i16)(
                    camera_word + ship_3d_camera_z_acceleration);
            ship_3d_camera_z_acceleration = (cb_u16)(
                    ship_3d_camera_z_acceleration
                    + CAMERA_APPROACH_Z_ACCELERATION_STEP);
        } else {
            sprite_slot_range_mark_dirty(
                    CAMERA_APPROACH_ENTITY_FIRST,
                    CAMERA_APPROACH_ENTITY_LAST);
            ++nav_camera_approach_phase;
        }
    } else if (phase == 3u) {
        nav_actor_presentation_state = 0xffffu;
        entity_flag_state_transition(4u);
        ship_3d_camera_z = (cb_i16)CAMERA_APPROACH_Z_CRUISE;
        ship_3d_projection_angle_a = 0u;
        ship_3d_camera_x = CAMERA_APPROACH_X_RESET;

        source = ship_3d_hyperspace_sequence_names[
                ship_3d_hyperspace_sequence_index
                & CAMERA_APPROACH_SEQUENCE_MASK];
        ++ship_3d_hyperspace_sequence_index;
        destination = ship_3d_hyperspace_filename_suffix;
        do {
            character = (cb_u8)*source++;
            *destination++ = (char)character;
        } while (character != 0u);

        vm_active_line = CAMERA_APPROACH_HYPERSPACE_LINE;
        ++nav_camera_approach_phase;
        return;
    } else if (phase == 4u) {
        dlg_line_id_scene_dispatch(scene_link_target);
        if ((vm_c2_presentation_gate & 1u) != 0u) {
            return;
        }

        nav_actor_presentation_state = 0u;
        entity_flag_state_transition(4u);
        ship_3d_hud_palette_snapshot_and_camera_reset();
        screen_flags_init();
        ++nav_camera_approach_phase;
        ship_3d_camera_z = (cb_i16)CAMERA_APPROACH_Z_FINAL;
        return;
    } else {
        camera_word = (cb_u16)ship_3d_camera_z;
        easing_step = (cb_u16)(0u - camera_word);
        easing_step >>= 2;
        if (easing_step != 0u) {
            ship_3d_camera_z = (cb_i16)(camera_word + easing_step);
        } else {
            ship_3d_camera_z_acceleration = 16u;
            ship_3d_camera_z = 0;
            nav_transition_pending = 0u;
            nav_camera_approach_phase = 0u;
            vm_ui_state.word &= (cb_u16)~CAMERA_APPROACH_UI_FLAG;
            screen_flags_init();
            nav_actor_presentation_state = 1u;
            return;
        }
    }

    blit_fill_row_5221(0u);
    ship_3d_projection_matrix_build();
    ship_3d_point_cloud_project();
    ship_3d_object_sprite_project();
}
