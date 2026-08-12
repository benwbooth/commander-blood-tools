/* Codegen probe for the MANU3 no-cursor per-frame coordinator. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA FAR
#endif

extern volatile u16 CODE_DATA active_data_segment;
extern volatile u16 framebuffer_window_offset;
extern volatile u16 framebuffer_segment;

extern void NEAR tween_step_probe(void);
extern void NEAR matrix_build_probe(void);
extern void NEAR entity_project_probe(void);
extern void NEAR face_builder_next_probe(void);

void FAR xdb_manu3_frame_step_probe(void);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_frame_step_probe \
        modify exact [ax bx cx dx si di bp]
#endif

void FAR xdb_manu3_frame_step_probe(void)
{
    if (active_data_segment == 0u) {
        return;
    }

    framebuffer_segment = (u16)(
            0xa000u + (framebuffer_window_offset >> 4));
    tween_step_probe();
    matrix_build_probe();
    entity_project_probe();
    face_builder_next_probe();
}
