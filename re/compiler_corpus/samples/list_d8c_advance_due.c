/* Codegen probe for BLOODPRG 0x00A240. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef u16 (FAR *audio_position_callback_type)(void);

extern volatile u8 presentation_mode_flag_27e0;
extern volatile u8 presentation_mode_flag_27e1;
extern volatile u8 voc_playback_enabled;
extern audio_position_callback_type audio_position_callback;
extern volatile u16 timer_tick_count;
extern volatile u16 list_d8c_audio_phase;
extern volatile u8 list_d8c_tick_threshold;
extern volatile u16 list_d8c_previous_tick;

int NEAR list_d8c_advance_due_probe(void)
{
    u16 current;
    u16 elapsed;

    if ((presentation_mode_flag_27e0 & 1u) != 0
            && (presentation_mode_flag_27e1 & 1u) != 0
            && (voc_playback_enabled & 1u) != 0) {
        current = audio_position_callback();
        current = (u16)(current - 0x4000u);
        current = (u16)(0u - current);
        elapsed = (u16)(current - list_d8c_audio_phase);
        if ((i16)elapsed < 0) {
            elapsed = (u16)(elapsed + 0x4000u);
        }
        if (elapsed < 0x0398u) {
            return 0;
        }
        list_d8c_audio_phase = current;
        return 1;
    }

    current = timer_tick_count;
    elapsed = (u16)(current - list_d8c_previous_tick);
    if ((i16)elapsed < 0) {
        elapsed = (u16)(0u - elapsed);
    }
    if ((elapsed & 0xff00u) == 0
            && (u8)elapsed < list_d8c_tick_threshold) {
        return 0;
    }
    list_d8c_previous_tick = timer_tick_count;
    return 1;
}
