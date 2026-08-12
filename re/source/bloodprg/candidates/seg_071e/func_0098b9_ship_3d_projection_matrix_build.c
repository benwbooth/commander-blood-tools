#include "../include/bloodprg_ship3d.h"

static cb_i32 ship_3d_angle_component(cb_i16 value)
{
    return ((cb_i32)value) + ((cb_i32)value);
}

static cb_i32 ship_3d_mul_shift_15(cb_i32 lhs, cb_i32 rhs)
{
    return (lhs * rhs) >> 15;
}

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
    const ship_3d_angle_table_entry *entry;

    entry = &ship_3d_angle_table[ship_3d_projection_angle_a];
    a_cos = ship_3d_angle_component(entry->cosine);
    a_sin = ship_3d_angle_component(entry->sine);

    entry = &ship_3d_angle_table[ship_3d_projection_angle_b];
    b_cos = ship_3d_angle_component(entry->cosine);
    b_sin = ship_3d_angle_component(entry->sine);

    entry = &ship_3d_angle_table[ship_3d_projection_angle_c];
    c_cos = ship_3d_angle_component(entry->cosine);
    c_sin = ship_3d_angle_component(entry->sine);

    b_sin_c_sin = ship_3d_mul_shift_15(b_sin, c_sin);
    c_sin_b_cos = ship_3d_mul_shift_15(c_sin, b_cos);

    ship_3d_projection.matrix[0] =
            ((a_cos * b_cos) + (b_sin_c_sin * a_sin)) >> 15;
    ship_3d_projection.matrix[1] = (-(c_cos * a_sin)) >> 15;
    ship_3d_projection.matrix[2] =
            ((c_sin_b_cos * a_sin) - (a_cos * b_sin)) >> 15;
    ship_3d_projection.matrix[3] =
            ((b_sin_c_sin * a_cos) - (a_sin * b_cos)) >> 15;
    ship_3d_projection.matrix[4] = -ship_3d_mul_shift_15(c_cos, a_cos);
    ship_3d_projection.matrix[5] =
            ((b_sin * a_sin) + (c_sin_b_cos * a_cos)) >> 15;
    ship_3d_projection.matrix[6] = ship_3d_mul_shift_15(b_sin, c_cos);
    ship_3d_projection.matrix[7] = c_sin;
    ship_3d_projection.matrix[8] = ship_3d_mul_shift_15(c_cos, b_cos);
}
