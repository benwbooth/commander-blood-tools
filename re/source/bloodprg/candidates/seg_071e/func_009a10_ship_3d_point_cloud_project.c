#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

#define SHIP_3D_MATRIX_SHIFT 15
#define SHIP_3D_AXIS_SHIFT 7
#define SHIP_3D_SCREEN_CENTER_X 160L
#define SHIP_3D_SCREEN_CENTER_Y 100L

void CB_FAR ship_3d_point_cloud_project(void)
{
    volatile ship_3d_point_record CB_GAME_DATA *point;
    volatile cb_u8 CB_FAR *framebuffer;
    cb_i32 axis;
    cb_i32 depth;
    cb_i16 x;
    cb_i16 y;
    cb_i16 z;

    ship_3d_projection_remaining = SHIP_3D_POINT_CLOUD_COUNT;
    point = ship_3d_point_cloud;
    framebuffer = graphics_display_buffer;

    do {
        ship_3d_projection_work = *point++;
        ship_3d_projection_work.x = (cb_u16)(
                ship_3d_projection_work.x - (cb_u16)ship_3d_camera_x);
        ship_3d_projection_work.y = (cb_u16)(
                ship_3d_projection_work.y - (cb_u16)ship_3d_camera_y);
        ship_3d_projection_work.z = (cb_u16)(
                ship_3d_projection_work.z - (cb_u16)ship_3d_camera_z);

        x = (cb_i16)ship_3d_projection_work.x;
        y = (cb_i16)ship_3d_projection_work.y;
        z = (cb_i16)ship_3d_projection_work.z;
        depth = (((cb_i32)x * ship_3d_projection.matrix[6])
                + ((cb_i32)y * ship_3d_projection.matrix[7])
                + ((cb_i32)z * ship_3d_projection.matrix[8]))
                >> SHIP_3D_MATRIX_SHIFT;

        if (depth > 0L) {
            axis = (((cb_i32)x * ship_3d_projection.matrix[0])
                    + ((cb_i32)y * ship_3d_projection.matrix[1])
                    + ((cb_i32)z * ship_3d_projection.matrix[2]))
                    >> SHIP_3D_AXIS_SHIFT;
            ship_3d_projection.projected_x = (cb_u16)(
                    (axis / depth) + SHIP_3D_SCREEN_CENTER_X);

            axis = (((cb_i32)x * ship_3d_projection.matrix[3])
                    + ((cb_i32)y * ship_3d_projection.matrix[4])
                    + ((cb_i32)z * ship_3d_projection.matrix[5]))
                    >> SHIP_3D_AXIS_SHIFT;
            ship_3d_projection.projected_y = (cb_u16)(
                    (axis / depth) + SHIP_3D_SCREEN_CENTER_Y);
            ship_3d_projection.projected_depth = (cb_u16)depth;
            ship_3d_plot_point(&ship_3d_projection, framebuffer);
        }
    } while (--ship_3d_projection_remaining != 0u);
}
