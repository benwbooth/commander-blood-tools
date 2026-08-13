#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_random.h"
#include "../include/bloodprg_vm.h"

void CB_FAR audio_process_ade(void)
{
    const cb_u16 CB_FAR *word_offsets;
    const char CB_FAR *word;
    cb_u16 selection_state;
    cb_u16 clip_index;
    cb_u16 word_count;
    cb_u16 hash;
    cb_u8 delay_step;
    cb_u8 delay;

    if ((voc_playback_enabled & 1u) == 0) {
        return;
    }

    if ((game_mode_0adf & 1u) == 0) {
        if ((vm_text_mode_0cf9 & 1u) != 0) {
            vm_text_mode_0cf9 = 0;
            word_offsets = vm_text_menu_words;
            hash = 0;
            word_count = 0;

            for (;;) {
                clip_index = *word_offsets++;
                if (clip_index == 0 || clip_index == 0xffffu) {
                    break;
                }

                word = vm_dic_words + clip_index;
                while (*word != '\0') {
                    hash = (cb_u16)(hash + (cb_i8)*word++);
                }
                ++word_count;
            }

            snd_dialogue_seed = (cb_u16)((hash + word_count) >> 4);
            vm_text_mode_0cfa = 1;
        } else if ((vm_text_mode_0cfa & 1u) != 0 &&
                snd_dialogue_delay == 0) {
            delay_step = (cb_u8)snd_dialogue_seed & 0x0fu;
            do {
                delay = (cb_u8)(snd_bank_header.dialogue_delay_base +
                        delay_step);
                delay_step >>= 1;
            } while (delay > snd_bank_header.dialogue_delay_limit);
            snd_dialogue_delay = (cb_i8)delay;

            selection_state = snd_dialogue_seed;
            clip_index = snd_dialogue_seed;
            for (;;) {
                selection_state = (cb_u16)(selection_state - 2u);
                clip_index = (cb_u16)(clip_index -
                        (selection_state & 0x001fu));
                if ((cb_i16)clip_index < 0) {
                    clip_index = (cb_u16)(0u - clip_index);
                }
                if (clip_index >= snd_streamed_clip_count) {
                    continue;
                }

                ++snd_dialogue_seed;
                if (clip_index != snd_last_clip) {
                    break;
                }
            }

            snd_last_clip = clip_index;
            snd_play_clip((cb_i16)(clip_index | 0x8000u));
        }
    }

    if ((vm_text_voice_trigger & 1u) != 0 &&
            snd_chatter_cooldown == 0) {
        snd_chatter_cooldown = 4;
        do {
            clip_index = blood_prng_next(10u);
        } while (clip_index == snd_last_clip);

        snd_last_clip = clip_index;
        snd_play_clip((cb_i16)(clip_index + 7u));
    }
}
