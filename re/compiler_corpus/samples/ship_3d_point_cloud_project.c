/* Codegen probe for BLOODPRG 0x009A10. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define FAR
#define NEAR
#define GAME_DATA FAR
#endif

#define POINT_COUNT 1000u

typedef struct projection_context {
    i32 matrix[9];
    u16 projected_x;
    u16 projected_y;
    u16 projected_depth;
    u16 depth_scale;
} projection_context;

typedef struct point_record {
    u16 x;
    u16 y;
    u16 z;
    u16 scratch;
} point_record;

typedef volatile u8 FAR *buffer_ptr;

extern volatile projection_context GAME_DATA projection;
extern volatile point_record GAME_DATA points[];
extern volatile point_record GAME_DATA projection_work;
extern volatile u16 GAME_DATA projection_remaining;
extern volatile i16 GAME_DATA camera_x;
extern volatile i16 GAME_DATA camera_y;
extern volatile i16 GAME_DATA camera_z;
extern buffer_ptr GAME_DATA display_buffer;

void NEAR plot_probe(
        const volatile projection_context GAME_DATA *context,
        volatile u8 FAR *framebuffer);

#if defined(__WATCOMC__)
#pragma aux ship_3d_point_cloud_project_probe modify exact []
#endif

void FAR ship_3d_point_cloud_project_probe(void)
{
    volatile point_record GAME_DATA *point;
    volatile u8 FAR *framebuffer;
    i32 axis;
    i32 depth;
    i16 x;
    i16 y;
    i16 z;

    projection_remaining = POINT_COUNT;
    point = points;
    framebuffer = display_buffer;

    do {
        projection_work = *point++;
        projection_work.x = (u16)(projection_work.x - (u16)camera_x);
        projection_work.y = (u16)(projection_work.y - (u16)camera_y);
        projection_work.z = (u16)(projection_work.z - (u16)camera_z);

        x = (i16)projection_work.x;
        y = (i16)projection_work.y;
        z = (i16)projection_work.z;
        depth = (((i32)x * projection.matrix[6])
                + ((i32)y * projection.matrix[7])
                + ((i32)z * projection.matrix[8])) >> 15;

        if (depth > 0L) {
            axis = (((i32)x * projection.matrix[0])
                    + ((i32)y * projection.matrix[1])
                    + ((i32)z * projection.matrix[2])) >> 7;
            projection.projected_x = (u16)((axis / depth) + 160L);

            axis = (((i32)x * projection.matrix[3])
                    + ((i32)y * projection.matrix[4])
                    + ((i32)z * projection.matrix[5])) >> 7;
            projection.projected_y = (u16)((axis / depth) + 100L);
            projection.projected_depth = (u16)depth;
            plot_probe(&projection, framebuffer);
        }
    } while (--projection_remaining != 0u);
}
