#include "../include/bloodprg_input.h"
#include "../include/bloodprg_random.h"
#include "../include/bloodprg_ship3d.h"

#define SHIP_3D_TRANSITION_ACTIVE 0x01u
#define SHIP_3D_OPEN_IDLE_THRESHOLD 120u
#define SHIP_3D_OPEN_STEP 4u
#define SHIP_3D_CLOSE_STEP 8u
#define SHIP_3D_CLOSE_RANDOM_MODULUS 20u

void CB_NEAR ship_3d_transition_state_update(void)
{
    if ((ship_3d_transition_armed & SHIP_3D_TRANSITION_ACTIVE) == 0u) {
        if (mouse_motion_idle_counter_ds <= SHIP_3D_OPEN_IDLE_THRESHOLD) {
            return;
        }

        ship_3d_depth_step = SHIP_3D_OPEN_STEP;
        ship_3d_depth_opening = SHIP_3D_TRANSITION_ACTIVE;
        ship_3d_transition_armed = SHIP_3D_TRANSITION_ACTIVE;
        return;
    }

    if (mouse_motion_idle_counter_ds != 0u) {
        if ((ship_3d_depth_opening & SHIP_3D_TRANSITION_ACTIVE) != 0u) {
            return;
        }
        if (blood_prng_next(SHIP_3D_CLOSE_RANDOM_MODULUS) != 0u) {
            return;
        }
    }

    ship_3d_depth_step = SHIP_3D_CLOSE_STEP;
    ship_3d_depth_closing = SHIP_3D_TRANSITION_ACTIVE;
    ship_3d_transition_armed = 0u;
}
