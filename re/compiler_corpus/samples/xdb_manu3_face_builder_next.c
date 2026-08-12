/* Codegen probe for the MANU3 geometry/raster segment selector. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct segment_directory {
    u16 field_000;
    u16 work_segment_0;
    u16 work_segment_1;
    u16 work_segment_2;
} segment_directory;

extern volatile segment_directory segments;
extern void NEAR face_bucket_sort_probe(
        u16 geometry_segment,
        u16 raster_segment);

void NEAR xdb_manu3_face_builder_next_probe(void);

#if defined(__WATCOMC__)
#pragma aux face_bucket_sort_probe \
        parm [ax] [dx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_face_builder_next_probe \
        modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_face_builder_next_probe(void)
{
    face_bucket_sort_probe(
            segments.work_segment_0,
            segments.work_segment_2);
}
