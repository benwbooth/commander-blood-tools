#include <stdio.h>

#include "bloodprg_graphics.h"

#define PALETTE_BYTES 768u

volatile cb_u8 palette_dirty;
volatile cb_u8 CB_GAME_DATA pbm_live_palette[PALETTE_BYTES];
cb_u8 palette_transition_source[PALETTE_BYTES];
cb_u8 CB_GAME_DATA palette_transition_target[PALETTE_BYTES];
volatile cb_u16 palette_transition_increment;
volatile cb_u16 palette_transition_percent;
volatile cb_u8 palette_transition_first;
volatile cb_u8 palette_transition_last;

static int write_result(const char *text)
{
    FILE *result = fopen("RESULT.TXT", "wt");

    if (result == NULL) {
        return 1;
    }
    fputs(text, result);
    fputc('\n', result);
    fclose(result);
    return text[0] == 'P' ? 0 : 1;
}

static cb_u8 expected_component(cb_u8 source, cb_u8 target, cb_i8 percent)
{
    cb_i8 delta = (cb_i8)(source - target);

    return (cb_u8)(target + (cb_i16)delta * (cb_i16)percent / 100);
}

int main(void)
{
    cb_u16 index;
    cb_u16 first_component;
    cb_u16 last_component;

    for (index = 0u; index < PALETTE_BYTES; ++index) {
        palette_transition_source[index] = (cb_u8)((index * 7u + 3u) & 63u);
        palette_transition_target[index] = (cb_u8)((index * 11u + 5u) & 63u);
        pbm_live_palette[index] = 0xa5u;
    }

    palette_dirty = 0u;
    palette_transition_percent = 20u;
    palette_transition_increment = 35u;
    palette_transition_first = 7u;
    palette_transition_last = 19u;
    palette_transition_step();

    if (palette_transition_percent != 55u || palette_dirty != 1u) {
        return write_result("FAIL palette transition state");
    }

    first_component = (cb_u16)(palette_transition_first * 3u);
    last_component = (cb_u16)((palette_transition_last + 1u) * 3u);
    for (index = 0u; index < PALETTE_BYTES; ++index) {
        cb_u8 expected = 0xa5u;

        if (index >= first_component && index < last_component) {
            expected = expected_component(
                    palette_transition_source[index],
                    palette_transition_target[index],
                    55);
        }
        if (pbm_live_palette[index] != expected) {
            return write_result("FAIL palette interpolation bytes");
        }
    }

    palette_dirty = 0u;
    palette_transition_percent = 90u;
    palette_transition_increment = 20u;
    palette_transition_step();
    if (palette_transition_percent != 100u || palette_dirty != 1u) {
        return write_result("FAIL palette transition clamp");
    }

    return write_result("PASS bloodprg palette transition ABI");
}
