/*
 * Codegen probe for BLOODPRG 0x0098B9.
 * This is not recovered game source.
 */
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

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

typedef struct projection_context {
    i32 matrix[9];
} projection_context;

typedef struct projection_terms {
    i32 b_cos;
    i32 b_sin;
    i32 c_cos;
    i32 c_sin;
    i32 a_cos;
    i32 a_sin;
} projection_terms;

typedef struct angle_entry {
    i16 cosine;
    i16 sine;
} angle_entry;

extern volatile u16 GAME_DATA projection_angle_b;
extern volatile u16 GAME_DATA projection_angle_c;
extern volatile u16 GAME_DATA projection_angle_a;
extern volatile projection_terms GAME_DATA projection_inputs;
extern volatile projection_context GAME_DATA projection;
extern const angle_entry GAME_DATA angle_table[];

#if defined(__WATCOMC__)
#pragma aux ship_3d_projection_matrix_build_probe modify exact [ax es]
#endif

void FAR ship_3d_projection_matrix_build_probe(void)
{
    i32 a_cos;
    i32 a_sin;
    i32 b_cos;
    i32 b_sin;
    i32 c_cos;
    i32 c_sin;
    i32 b_sin_c_sin;
    i32 c_sin_b_cos;
    const angle_entry GAME_DATA *entry;

    entry = &angle_table[projection_angle_a];
    a_cos = (i32)entry->cosine * 2L;
    a_sin = (i32)entry->sine * 2L;
    projection_inputs.a_cos = a_cos;
    projection_inputs.a_sin = a_sin;

    entry = &angle_table[projection_angle_b];
    b_cos = (i32)entry->cosine * 2L;
    b_sin = (i32)entry->sine * 2L;
    projection_inputs.b_cos = b_cos;
    projection_inputs.b_sin = b_sin;

    entry = &angle_table[projection_angle_c];
    c_cos = (i32)entry->cosine * 2L;
    c_sin = (i32)entry->sine * 2L;
    projection_inputs.c_cos = c_cos;
    projection_inputs.c_sin = c_sin;

    b_sin_c_sin = (b_sin * c_sin) >> 15;
    c_sin_b_cos = (c_sin * b_cos) >> 15;

    projection.matrix[0] =
            ((a_cos * b_cos) + (b_sin_c_sin * a_sin)) >> 15;
    projection.matrix[1] = (-(c_cos * a_sin)) >> 15;
    projection.matrix[2] =
            ((c_sin_b_cos * a_sin) - (a_cos * b_sin)) >> 15;
    projection.matrix[3] =
            ((b_sin_c_sin * a_cos) - (a_sin * b_cos)) >> 15;
    projection.matrix[4] = -((c_cos * a_cos) >> 15);
    projection.matrix[5] =
            ((b_sin * a_sin) + (c_sin_b_cos * a_cos)) >> 15;
    projection.matrix[6] = (b_sin * c_cos) >> 15;
    projection.matrix[7] = c_sin;
    projection.matrix[8] = (c_cos * b_cos) >> 15;
}
