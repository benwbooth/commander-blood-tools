/*
 * Codegen probe for BLOODPRG 0x00B7E3.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct snd_bank_header {
    u16 clip_count;
    u8 dialogue_delay_base;
    u8 dialogue_delay_limit;
} snd_bank_header;

extern volatile u8 voc_playback_enabled;
extern volatile u8 game_mode_0adf;
extern volatile u8 text_mode_cf9;
extern volatile u8 text_mode_cfa;
extern volatile u8 text_voice_trigger;
extern volatile u8 chatter_cooldown;
extern volatile u16 dialogue_delay;
extern volatile u16 last_clip;
extern volatile u16 dialogue_seed;
extern volatile u16 streamed_clip_count;
extern volatile snd_bank_header bank_header;
extern const char FAR *dictionary_words;
extern const u16 FAR * volatile text_menu_words;

u16 FAR prng_probe(u16 modulus);
void FAR play_clip_probe(i16 clip_index);

#if defined(__WATCOMC__)
#pragma aux prng_probe parm [ax] value [ax] modify exact [ax]
#pragma aux play_clip_probe parm [ax] modify exact []
#endif

void FAR audio_process_ade_probe(void)
{
    const u16 FAR *word_offsets;
    const char FAR *word;
    u16 selection_state;
    u16 clip_index;
    u16 word_count;
    u16 hash;
    u8 delay_step;
    u8 delay;

    if ((voc_playback_enabled & 1u) == 0) {
        return;
    }

    if ((game_mode_0adf & 1u) == 0) {
        if ((text_mode_cf9 & 1u) != 0) {
            text_mode_cf9 = 0;
            word_offsets = text_menu_words;
            hash = 0;
            word_count = 0;

            for (;;) {
                clip_index = *word_offsets++;
                if (clip_index == 0 || clip_index == 0xffffu) {
                    break;
                }

                word = dictionary_words + clip_index;
                while (*word != '\0') {
                    hash = (u16)(hash + (i8)*word++);
                }
                ++word_count;
            }

            dialogue_seed = (u16)((hash + word_count) >> 4);
            text_mode_cfa = 1;
        } else if ((text_mode_cfa & 1u) != 0 && dialogue_delay == 0) {
            delay_step = (u8)dialogue_seed & 0x0fu;
            do {
                delay = (u8)(bank_header.dialogue_delay_base + delay_step);
                delay_step >>= 1;
            } while (delay > bank_header.dialogue_delay_limit);
            dialogue_delay = (i8)delay;

            selection_state = dialogue_seed;
            clip_index = dialogue_seed;
            for (;;) {
                selection_state = (u16)(selection_state - 2u);
                clip_index = (u16)(clip_index - (selection_state & 0x001fu));
                if ((i16)clip_index < 0) {
                    clip_index = (u16)(0u - clip_index);
                }
                if (clip_index >= streamed_clip_count) {
                    continue;
                }

                ++dialogue_seed;
                if (clip_index != last_clip) {
                    break;
                }
            }

            last_clip = clip_index;
            play_clip_probe((i16)(clip_index | 0x8000u));
        }
    }

    if ((text_voice_trigger & 1u) != 0 && chatter_cooldown == 0) {
        chatter_cooldown = 4;
        do {
            clip_index = prng_probe(10u);
        } while (clip_index == last_clip);

        last_clip = clip_index;
        play_clip_probe((i16)(clip_index + 7u));
    }
}
