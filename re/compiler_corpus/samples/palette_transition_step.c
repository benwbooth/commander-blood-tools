/* Codegen probe for BLOODPRG 0x001F78. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

#define TRANSITION_COMPLETE 100u

extern u8 transition_source[768];
extern u8 GAME_DATA transition_target[768];
extern volatile u16 transition_increment;
extern volatile u16 transition_percent;
extern volatile u8 transition_first;
extern volatile u8 transition_last;
extern volatile u8 palette_dirty_probe;

void FAR palette_range_interpolate_ds_probe(
        const u8 *source,
        const u8 FAR *target,
        u16 percent,
        u16 first,
        u16 last);
void FAR palette_transition_step_probe(void);

#if defined(__WATCOMC__)
#pragma aux palette_range_interpolate_ds_probe "palette_range_interpolate_probe_" \
        parm [si] [es di] [ax] [bx] [dx] \
        modify exact []
#pragma aux palette_transition_step_probe modify exact []
#endif

void FAR palette_transition_step_probe(void)
{
    u16 first;
    u16 last;
    u16 percent;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    percent = transition_percent;
    if (percent != TRANSITION_COMPLETE) {
        percent = (u16)(percent + transition_increment);
        if ((i16)percent > (i16)TRANSITION_COMPLETE) {
            percent = TRANSITION_COMPLETE;
        }

        palette_dirty_probe = 1u;
        transition_percent = percent;
        first = (u16)transition_first;
        last = (u16)transition_last;
        palette_range_interpolate_ds_probe(
            transition_source,
            transition_target,
            percent,
            first,
            last);
    }

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
