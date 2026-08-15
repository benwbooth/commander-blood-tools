#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_nav.h"

int CB_NEAR list_d8c_advance_due(void)
{
    cb_u16 current;
    cb_u16 elapsed;

    if ((presentation_mode_flag_27e0 & 1u) != 0
            && (presentation_mode_flag_27e1 & 1u) != 0
            && (voc_playback_enabled & 1u) != 0) {
        current = audio_position_callback();
        current = (cb_u16)(current - 0x4000u);
        current = (cb_u16)(0u - current);
        elapsed = (cb_u16)(current - list_d8c_audio_phase);
        if ((cb_i16)elapsed < 0) {
            elapsed = (cb_u16)(elapsed + 0x4000u);
        }
        if (elapsed < 0x0398u) {
            return 0;
        }
        list_d8c_audio_phase = current;
        return 1;
    }

    current = timer_tick_count_low;
    elapsed = (cb_u16)(current - list_d8c_previous_tick);
    if ((cb_i16)elapsed < 0) {
        elapsed = (cb_u16)(0u - elapsed);
    }
    if ((elapsed & 0xff00u) == 0
            && (cb_u8)elapsed < list_d8c_tick_threshold) {
        return 0;
    }
    list_d8c_previous_tick = timer_tick_count_low;
    return 1;
}
