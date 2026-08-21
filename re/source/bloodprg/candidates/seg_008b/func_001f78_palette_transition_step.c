#include "../include/bloodprg_graphics.h"

#define PALETTE_TRANSITION_COMPLETE 100u

void CB_SAVE_REGS CB_FAR palette_transition_step(void)
{
    cb_u16 first;
    cb_u16 last;
    cb_u16 percent;

    percent = palette_transition_percent;
    if (percent != PALETTE_TRANSITION_COMPLETE) {
        percent = (cb_u16)(percent + palette_transition_increment);
        if ((cb_i16)percent > (cb_i16)PALETTE_TRANSITION_COMPLETE) {
            percent = PALETTE_TRANSITION_COMPLETE;
        }

        palette_dirty = 1u;
        palette_transition_percent = percent;
        first = (cb_u16)palette_transition_first;
        last = (cb_u16)palette_transition_last;
        palette_range_interpolate(
            palette_transition_source,
            palette_transition_target,
            percent,
            first,
            last);
    }
}
