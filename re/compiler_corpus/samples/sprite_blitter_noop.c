/* Codegen probe for BLOODPRG 0x00509A..0x00509C. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct sprite_slot_probe {
    u16 words[16];
} sprite_slot_probe;

#if defined(__WATCOMC__)
#pragma aux sprite_blitter_noop_probe parm [di] modify exact []
#endif

void NEAR sprite_blitter_noop_probe(volatile sprite_slot_probe *record)
{
    (void)record;
}
