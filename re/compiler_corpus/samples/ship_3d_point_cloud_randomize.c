/*
 * Codegen probe for BLOODPRG 0x009B67.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define GAME_DATA far
#else
#define GAME_DATA
#endif

typedef struct point_record {
    u16 x;
    u16 y;
    u16 z;
    u16 scratch;
} point_record;

extern volatile point_record GAME_DATA point_cloud[];
u16 FAR blood_prng_next(u16 modulus);

#if defined(__WATCOMC__)
#pragma aux blood_prng_next parm [ax] value [ax] modify exact [ax]
#pragma aux ship_3d_point_cloud_randomize_probe modify exact [ax cx es]
#endif

void FAR ship_3d_point_cloud_randomize_probe(void)
{
    volatile point_record GAME_DATA *point;

    point = point_cloud;
    do {
        point->x = blood_prng_next(0xffffu);
        point->y = blood_prng_next(0xffffu);
        point->z = blood_prng_next(0xffffu);
        ++point;
    } while (point != point_cloud + 1000u);
}
