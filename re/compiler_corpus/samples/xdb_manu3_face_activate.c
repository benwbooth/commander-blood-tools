/* Codegen probe for the MANU3 face-to-gradient activation prelude. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct face_record {
    u16 link;
    u16 vertex_0;
    u16 vertex_1;
    u16 vertex_2;
} face_record;

extern volatile u16 active_raster_offset;
extern void NEAR gradient_setup_probe(
        u16 vertex_0,
        u16 vertex_1,
        u16 vertex_2,
        volatile void NEAR *raster);

void NEAR xdb_manu3_face_activate_probe(
        const volatile face_record FAR *face);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_face_activate_probe \
        parm [es si] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_face_activate_probe(
        const volatile face_record FAR *face)
{
    u16 vertex_0 = face->vertex_0;
    u16 vertex_1 = face->vertex_1;
    u16 vertex_2 = face->vertex_2;
    volatile void NEAR *raster = (volatile void NEAR *)active_raster_offset;

    if (raster != 0) {
        gradient_setup_probe(vertex_0, vertex_1, vertex_2, raster);
    }
}
