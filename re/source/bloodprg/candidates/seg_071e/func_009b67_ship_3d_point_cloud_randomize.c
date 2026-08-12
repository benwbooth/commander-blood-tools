#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

void CB_FAR ship_3d_point_cloud_randomize(void)
{
    volatile ship_3d_point_record CB_GAME_DATA *point;

    point = ship_3d_point_cloud;
    do {
        point->x = (cb_u16)blood_prng_next(0xffffu);
        point->y = (cb_u16)blood_prng_next(0xffffu);
        point->z = (cb_u16)blood_prng_next(0xffffu);
        ++point;
    } while (point != ship_3d_point_cloud + 1000u);
}
