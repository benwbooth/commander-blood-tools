/*
 * Codegen probe for SCRUT XDB alien method-table slot 12.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct alien_biased_state {
    u8 field_000[0x52];
    volatile i16 field_052;
} alien_biased_state;

typedef struct alien_state {
    u8 field_000[0x0b0];
    volatile i16 field_0b0;
} alien_state;

typedef struct alien_method_context {
    u8 field_00[0x16];
    volatile alien_state NEAR *state;
} alien_method_context;

volatile u8 NEAR *NEAR xdb_lower_state_probe(
        const alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_lower_state_probe parm [di] value [si] modify exact [si]
#endif

volatile u8 NEAR *NEAR xdb_lower_state_probe(
        const alien_method_context NEAR *context)
{
    volatile alien_state NEAR *state;
    volatile alien_biased_state NEAR *biased;

    state = context->state;
    biased = (volatile alien_biased_state NEAR *)
        ((volatile u8 NEAR *)state + 0x005eu);
    biased->field_052 = (i16)(biased->field_052 - 0x000f);
    return (volatile u8 NEAR *)biased;
}
