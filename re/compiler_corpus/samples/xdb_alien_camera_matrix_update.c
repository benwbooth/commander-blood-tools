/* Codegen probe for the shared alien camera-matrix update. */
typedef unsigned char xdb_u8;
typedef unsigned int xdb_u16;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;
typedef signed long xdb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define XDB_NEAR near
#else
#define XDB_NEAR
#endif

typedef struct xdb_alien_trig_sample {
    xdb_i16 cosine;
    xdb_i16 sine;
} xdb_alien_trig_sample;

extern volatile xdb_i16 xdb_alien_matrix_angle_pan;
extern volatile xdb_i16 xdb_alien_matrix_angle_pitch;
extern volatile xdb_i16 xdb_alien_matrix_angle_pan_secondary;
extern volatile xdb_alien_trig_sample xdb_alien_angle_table[];
extern volatile xdb_i16 xdb_alien_view_x;
extern volatile xdb_i16 xdb_alien_view_y;
extern volatile xdb_i16 xdb_alien_view_z;
extern volatile xdb_i16 xdb_alien_camera_pitch;
extern volatile xdb_i16 xdb_alien_camera_pan;
extern volatile xdb_i16 xdb_alien_camera_pan_secondary;
extern volatile xdb_i16 xdb_alien_camera_depth_step;
extern volatile xdb_i32 xdb_alien_camera_target_matrix[9];
extern volatile xdb_i32 xdb_alien_camera_matrix[9];
extern volatile xdb_i32 xdb_alien_camera_result[3];
extern volatile xdb_i32 xdb_alien_camera_position[3];

void XDB_NEAR xdb_alien_camera_matrix_update_probe(void);

#if defined(__WATCOMC__)
#pragma aux xdb_alien_camera_matrix_update_probe \
        modify exact [ax bx cx dx si di bp]
#endif


void XDB_NEAR xdb_alien_camera_matrix_update_probe(void)
{
    volatile xdb_alien_trig_sample XDB_NEAR *first;
    volatile xdb_alien_trig_sample XDB_NEAR *second;
    volatile xdb_alien_trig_sample XDB_NEAR *axis;
    xdb_u16 pitch;
    xdb_u16 pan;
    xdb_u16 secondary;
    xdb_u16 combined;
    xdb_u16 index;
    xdb_i32 cosine_half_difference;
    xdb_i32 sine_half_sum;
    xdb_i32 correction;
    xdb_u32 current;
    xdb_u32 delta;
    xdb_u32 product;
    xdb_u32 accumulator;
    xdb_i32 step;
    xdb_u32 depth_factor;
    xdb_i32 view_x;
    xdb_i32 view_y;
    xdb_i32 view_z;

    pitch = (xdb_u16)xdb_alien_camera_pitch & 0x0ffcu;
    pan = (xdb_u16)xdb_alien_camera_pan & 0x0ffcu;
    secondary = (xdb_u16)xdb_alien_camera_pan_secondary & 0x0ffcu;
    xdb_alien_matrix_angle_pan = (xdb_i16)pan;
    xdb_alien_matrix_angle_pitch = (xdb_i16)pitch;
    xdb_alien_matrix_angle_pan_secondary = (xdb_i16)secondary;

    xdb_alien_camera_target_matrix[7] = (xdb_i32)(
            0UL - ((xdb_u32)(xdb_i32)xdb_alien_angle_table[pitch >> 2].sine << 1));

    combined = (pan + secondary) & 0x0ffcu;
    first = xdb_alien_angle_table + (((pitch - combined) & 0x0ffcu) >> 2);
    second = xdb_alien_angle_table + (((pitch + combined) & 0x0ffcu) >> 2);
    cosine_half_difference = (xdb_i32)first->cosine - (xdb_i32)second->cosine;
    sine_half_sum = (xdb_i32)first->sine + (xdb_i32)second->sine;
    cosine_half_difference >>= 1;
    sine_half_sum >>= 1;
    axis = xdb_alien_angle_table + (combined >> 2);
    correction = cosine_half_difference + (xdb_i32)axis->sine;
    xdb_alien_camera_target_matrix[3] = correction;
    xdb_alien_camera_target_matrix[2] = (xdb_i32)(0UL - (xdb_u32)correction);
    correction = sine_half_sum + (xdb_i32)axis->cosine;
    xdb_alien_camera_target_matrix[0] = correction;
    xdb_alien_camera_target_matrix[5] = correction;

    combined = (pan - secondary) & 0x0ffcu;
    first = xdb_alien_angle_table + (((pitch - combined) & 0x0ffcu) >> 2);
    second = xdb_alien_angle_table + (((pitch + combined) & 0x0ffcu) >> 2);
    cosine_half_difference = (xdb_i32)first->cosine - (xdb_i32)second->cosine;
    sine_half_sum = (xdb_i32)first->sine + (xdb_i32)second->sine;
    cosine_half_difference >>= 1;
    sine_half_sum >>= 1;
    axis = xdb_alien_angle_table + (combined >> 2);
    correction = (xdb_i32)axis->sine - cosine_half_difference;
    xdb_alien_camera_target_matrix[3] = (xdb_i32)(
            (xdb_u32)xdb_alien_camera_target_matrix[3] - (xdb_u32)correction);
    xdb_alien_camera_target_matrix[2] = (xdb_i32)(
            (xdb_u32)xdb_alien_camera_target_matrix[2] - (xdb_u32)correction);
    correction = (xdb_i32)axis->cosine - sine_half_sum;
    xdb_alien_camera_target_matrix[0] = (xdb_i32)(
            (xdb_u32)xdb_alien_camera_target_matrix[0] + (xdb_u32)correction);
    xdb_alien_camera_target_matrix[5] = (xdb_i32)(
            (xdb_u32)xdb_alien_camera_target_matrix[5] - (xdb_u32)correction);

    first = xdb_alien_angle_table + (((secondary + pitch) & 0x0ffcu) >> 2);
    second = xdb_alien_angle_table + (((secondary - pitch) & 0x0ffcu) >> 2);
    xdb_alien_camera_target_matrix[4] =
            (xdb_i32)first->cosine + (xdb_i32)second->cosine;
    xdb_alien_camera_target_matrix[1] = (xdb_i32)(
            0UL - (xdb_u32)((xdb_i32)first->sine + (xdb_i32)second->sine));

    first = xdb_alien_angle_table + (((pan + pitch) & 0x0ffcu) >> 2);
    second = xdb_alien_angle_table + (((pan - pitch) & 0x0ffcu) >> 2);
    xdb_alien_camera_target_matrix[8] =
            (xdb_i32)first->cosine + (xdb_i32)second->cosine;
    xdb_alien_camera_target_matrix[6] =
            (xdb_i32)first->sine + (xdb_i32)second->sine;

    for (index = 0; index < 9u; ++index) {
        current = (xdb_u32)xdb_alien_camera_matrix[index];
        delta = (xdb_u32)xdb_alien_camera_target_matrix[index] - current;
        step = (xdb_i32)delta >> 3;
        xdb_alien_camera_matrix[index] = (xdb_i32)(
                current + (xdb_u32)step + ((delta >> 2) & 1UL));
    }

    depth_factor = 0UL - (xdb_u32)(xdb_i32)xdb_alien_camera_depth_step;
    for (index = 0; index < 3u; ++index) {
        product = (xdb_u32)xdb_alien_camera_matrix[index + 6u] * depth_factor;
        xdb_alien_camera_position[index] = (xdb_i32)(
                (xdb_u32)xdb_alien_camera_position[index] +
                (xdb_u32)((xdb_i32)product >> 3));
    }

    view_x = (xdb_i32)xdb_alien_view_x;
    view_y = (xdb_i32)xdb_alien_view_y;
    view_z = (xdb_i32)xdb_alien_view_z;
    for (index = 3u; index != 0u; --index) {
        xdb_u16 row = index - 1u;

        accumulator = (xdb_u32)xdb_alien_camera_matrix[row * 3u] * (xdb_u32)view_x;
        accumulator += (xdb_u32)xdb_alien_camera_matrix[row * 3u + 1u] *
                       (xdb_u32)view_y;
        accumulator += (xdb_u32)xdb_alien_camera_matrix[row * 3u + 2u] *
                       (xdb_u32)view_z;
        xdb_alien_camera_result[row] = (xdb_i32)accumulator;
    }
}
