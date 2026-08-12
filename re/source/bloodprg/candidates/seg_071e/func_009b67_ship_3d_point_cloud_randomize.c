#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

void CB_FAR ship_3d_point_cloud_randomize(void)
{
    cb_u16 i;

    for (i = 0; i < 1000u; ++i) {
        ship_3d_point_cloud[i].x = (cb_u16)blood_prng_next(0xffffu);
        ship_3d_point_cloud[i].y = (cb_u16)blood_prng_next(0xffffu);
        ship_3d_point_cloud[i].z = (cb_u16)blood_prng_next(0xffffu);
    }
}
