#include "../include/xdb_alien.h"

int XDB_NEAR xdb_amer_resume_pair_outside(
        xdb_alien_biased_state XDB_NEAR *current,
        xdb_alien_biased_state XDB_NEAR *other)
{
    xdb_i32 z_delta = (xdb_i32)(xdb_i16)other->position_z
            - (xdb_i32)(xdb_i16)current->position_z;
    xdb_i32 x_delta = (xdb_i32)(xdb_i16)other->position_x
            - (xdb_i32)(xdb_i16)current->position_x;
    xdb_i16 y_delta = (xdb_i16)(
            (xdb_u16)other->position_y - (xdb_u16)current->position_y);
    xdb_i16 y_step;
    xdb_i16 steering;

    if (z_delta >= -100 && z_delta <= 100
            && x_delta >= -200 && x_delta <= 200
            && y_delta >= -200 && y_delta < 200) {
        return 0;
    }

    y_step = (xdb_i16)(
            ((xdb_u16)y_delta >> 3) | ((xdb_u16)y_delta & 0xe000u));
    steering = (xdb_i16)(
            (xdb_u16)current->field_04e + (xdb_u16)(-y_step));
    current->field_04e = (xdb_i16)(
            ((xdb_u16)steering >> 1) | ((xdb_u16)steering & 0x8000u));

    {
        xdb_u16 sample_offset = current->field_050 & 0x0ffcu;
        volatile xdb_alien_trig_sample XDB_NEAR *sample =
                &xdb_alien_angle_table[sample_offset >> 2];
        xdb_i32 direction =
                (xdb_i32)sample->cosine * x_delta
                - (xdb_i32)sample->sine * z_delta;
        xdb_u16 step = direction < 0 ? 0xffe0u : 0x0010u;

        current->field_050 = (xdb_u16)(sample_offset + step);
    }
    return 1;
}
