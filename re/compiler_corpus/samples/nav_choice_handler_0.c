/*
 * Codegen probe for BLOODPRG 0x008713.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 choice_phase;
extern volatile u16 honk_record;
extern volatile u16 deferred_record_type;
extern volatile u16 deferred_record_link;

#if defined(__WATCOMC__)
#pragma aux nav_choice_handler_0_probe modify exact [ax]
#endif

void NEAR nav_choice_handler_0_probe(void)
{
    if ((choice_phase & 1u) == 0) {
        return;
    }

    deferred_record_link = honk_record;
    deferred_record_type = 0x00c3u;
    choice_phase = 0;
}
