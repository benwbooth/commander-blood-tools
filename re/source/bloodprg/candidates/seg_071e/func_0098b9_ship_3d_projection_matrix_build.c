#include "../include/bloodprg_ship3d.h"

void CB_FAR ship_3d_projection_matrix_build(void)
{
    cb_i32 a_cos;
    cb_i32 a_sin;
    cb_i32 b_cos;
    cb_i32 b_sin;
    cb_i32 c_cos;
    cb_i32 c_sin;
    cb_i32 b_sin_c_sin;
    cb_i32 c_sin_b_cos;
    const ship_3d_angle_table_entry CB_GAME_DATA *entry;

    entry = &ship_3d_angle_table[ship_3d_projection_angle_a];
    a_cos = (cb_i32)entry->cosine * 2L;
    a_sin = (cb_i32)entry->sine * 2L;
    ship_3d_projection_inputs.a_cos = a_cos;
    ship_3d_projection_inputs.a_sin = a_sin;

    entry = &ship_3d_angle_table[ship_3d_projection_angle_b];
    b_cos = (cb_i32)entry->cosine * 2L;
    b_sin = (cb_i32)entry->sine * 2L;
    ship_3d_projection_inputs.b_cos = b_cos;
    ship_3d_projection_inputs.b_sin = b_sin;

    entry = &ship_3d_angle_table[ship_3d_projection_angle_c];
    c_cos = (cb_i32)entry->cosine * 2L;
    c_sin = (cb_i32)entry->sine * 2L;
    ship_3d_projection_inputs.c_cos = c_cos;
    ship_3d_projection_inputs.c_sin = c_sin;

    b_sin_c_sin = (b_sin * c_sin) >> 15;
    c_sin_b_cos = (c_sin * b_cos) >> 15;

    ship_3d_projection.matrix[0] =
            ((a_cos * b_cos) + (b_sin_c_sin * a_sin)) >> 15;
    ship_3d_projection.matrix[1] = (-(c_cos * a_sin)) >> 15;
    ship_3d_projection.matrix[2] =
            ((c_sin_b_cos * a_sin) - (a_cos * b_sin)) >> 15;
    ship_3d_projection.matrix[3] =
            ((b_sin_c_sin * a_cos) - (a_sin * b_cos)) >> 15;
    ship_3d_projection.matrix[4] = -((c_cos * a_cos) >> 15);
    ship_3d_projection.matrix[5] =
            ((b_sin * a_sin) + (c_sin_b_cos * a_cos)) >> 15;
    ship_3d_projection.matrix[6] = (b_sin * c_cos) >> 15;
    ship_3d_projection.matrix[7] = c_sin;
    ship_3d_projection.matrix[8] = (c_cos * b_cos) >> 15;
}
