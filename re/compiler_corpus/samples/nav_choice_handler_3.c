/*
 * Codegen probe for BLOODPRG 0x008848.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 choice_phase;
extern volatile u16 radio_record;
extern volatile u16 deferred_record_type;
extern volatile u16 deferred_record_link;
extern volatile char radio_snd_path[];

void FAR snd_bank_loader_probe(u16 mode, const volatile char NEAR *path);

#if defined(__WATCOMC__)
#pragma aux snd_bank_loader_probe parm [ax] [si] modify exact []
#pragma aux nav_choice_handler_3_probe modify exact [ax si]
#endif

void NEAR nav_choice_handler_3_probe(void)
{
    if ((choice_phase & 1u) == 0) {
        return;
    }

    deferred_record_link = radio_record;
    deferred_record_type = 0x00c3u;
    choice_phase = 0;
    snd_bank_loader_probe(1u, radio_snd_path);
}
