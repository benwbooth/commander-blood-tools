/*
 * Codegen probe for AMER/CROOLIS XDB alien method-table slot 12.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#define FAR far
#else
#define NEAR
#define FAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA FAR
#endif

typedef struct alien_state {
    u8 field_000[0x0b0];
    volatile i16 field_0b0;
} alien_state;

typedef struct alien_method_context {
    u8 field_00[0x16];
    volatile alien_state NEAR *state;
} alien_method_context;

extern volatile i16 CODE_DATA method_delta;

i16 NEAR xdb_apply_delta_probe(
        const alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_apply_delta_probe parm [di] value [ax] modify exact [ax si]
#endif

i16 NEAR xdb_apply_delta_probe(
        const alien_method_context NEAR *context)
{
    volatile alien_state NEAR *state;
    i16 delta;

    state = context->state;
    if ((delta = (i16)(method_delta >> 1)) >= 0) {
        state->field_0b0 = (i16)(state->field_0b0 + delta);
    }
    return delta;
}
