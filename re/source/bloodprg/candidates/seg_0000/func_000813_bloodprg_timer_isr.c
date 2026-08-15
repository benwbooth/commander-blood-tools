#include <conio.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_hardware.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_vm.h"

#define BLOODPRG_BIOS_TIMER_DIVIDER 11u
#define BLOODPRG_GAME_SUBTICK_DIVIDER 25u
#define BLOODPRG_VM_TIMER_COUNT 30u

void CB_INTERRUPT CB_FAR bloodprg_timer_isr(void)
{
    cb_u16 tick_low;
    cb_u16 index;
    cb_u8 speaker_request;
    cb_u8 speaker_control;

    if ((timer_hook_active & 1u) == 0u) {
        timer_previous_handler();
        return;
    }

    if ((game_mode_0adf_gs & 1u) == 0u) {
        timer_periodic_update_ready = 0u;

        if (timer_state.frame_delay_ticks != 0u) {
            --timer_state.frame_delay_ticks;
        }

        ++timer_state.tick_count_low;
        tick_low = timer_state.tick_count_low;

        if ((tick_low & 1u) == 0u) {
            if (timer_state.chatter_cooldown != 0u) {
                --timer_state.chatter_cooldown;
            }

            if ((tick_low & 3u) == 0u) {
                if (timer_state.subtitle_reveal_delay != 0u) {
                    --timer_state.subtitle_reveal_delay;
                }

                if ((tick_low & 7u) == 0u) {
                    if (timer_state.dialogue_delay != 0u) {
                        --timer_state.dialogue_delay;
                    }

                    --timer_subtick_limit;
                    if (timer_subtick_limit == 0u) {
                        ++timer_state.mouse_motion_idle_counter;

                        if (nav_pending_record_link_gs == 0u) {
                            for (index = 0u;
                                    index < BLOODPRG_VM_TIMER_COUNT;
                                    ++index) {
                                if ((cb_i16)vm_state_words_gs[index] > 0) {
                                    --vm_state_words_gs[index];
                                }
                            }
                        }

                        timer_subtick_limit = BLOODPRG_GAME_SUBTICK_DIVIDER;
                        if (timer_state.clip_playback_state != 0u) {
                            --timer_state.clip_playback_state;
                        }
                    }

                    if ((tick_low & 15u) == 0u) {
                        ++nav_chart_entity_state_mask_gs;
                        if (timer_state.dialogue_hold_countdown != 0u) {
                            --timer_state.dialogue_hold_countdown;
                        }

                        if ((tick_low & 31u) == 0u) {
                            speaker_request = snd_clip_enable_request_gs;
                            if ((speaker_request & 1u) != 0u) {
                                speaker_control = (cb_u8)inp(0x0061u);
                                if ((speaker_request & 2u) == 0u) {
                                    speaker_control |= 3u;
                                    snd_clip_enable_request_gs =
                                            speaker_request | 2u;
                                } else {
                                    speaker_control &= 0xfcu;
                                    snd_clip_enable_request_gs = 0u;
                                }
                                outp(0x0061u, speaker_control);
                            }

                            timer_periodic_update_ready = 1u;
                            if (timer_state.subtitle_opening_frame_pulse != 0u) {
                                --timer_state.subtitle_opening_frame_pulse;
                            }
                        }
                    }
                }
            }
        }
    }

    --timer_divider;
    if (timer_divider == 0u) {
        timer_divider = BLOODPRG_BIOS_TIMER_DIVIDER;
        timer_previous_handler();
        return;
    }

    outp(0x0020u, 0x20u);
}
