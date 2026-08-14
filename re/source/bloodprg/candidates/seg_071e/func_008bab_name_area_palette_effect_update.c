#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_random.h"

#define NAME_AREA_SCREEN_WIDTH 320u
#define NAME_AREA_PALETTE_BASE 0xe0u
#define NAME_AREA_PALETTE_LAST 0xefu
#define NAME_AREA_RANDOM_SEQUENCE_COUNT 9u

void CB_NEAR name_area_palette_effect_update(void)
{
    const bloodprg_name_area_effect_sequence CB_NEAR *sequence;
    const bloodprg_name_area_effect_frame CB_NEAR *frame;
    volatile cb_u8 CB_FAR *pixel;
    cb_u16 columns;
    cb_u16 row_offset;
    cb_u16 row_skip;
    cb_u16 rows;
    cb_u8 operation;
    cb_u8 palette_index;
    cb_u8 replacement;

    if ((name_area_effect_active_ds & 1u) == 0u) {
        return;
    }

    if ((name_area_effect_restart & 1u) != 0u) {
        sequence = name_area_effect_sequences[0];
        name_area_effect_control.word = sequence->control.word;
        name_area_effect_frame_cursor = sequence->frames;
        name_area_effect_restart = 0u;
    }

    if (name_area_effect_control.fields.frames_remaining == 0u) {
        sequence = name_area_effect_sequences[
                blood_prng_next(NAME_AREA_RANDOM_SEQUENCE_COUNT) + 1u];
        name_area_effect_control.word = sequence->control.word;
        name_area_effect_frame_cursor = sequence->frames;
    }
    --name_area_effect_control.fields.frames_remaining;

    frame = name_area_effect_frame_cursor;
    name_area_effect_frame_cursor = frame + 1;
    operation = name_area_effect_control.fields.operation;
    row_offset = (cb_u16)((frame->y << 8) | (frame->y >> 8));
    row_offset = (cb_u16)(row_offset + (frame->y << 6));
    pixel = graphics_display_buffer_ds
            + row_offset + frame->x;
    row_skip = (cb_u16)(NAME_AREA_SCREEN_WIDTH - frame->width);
    rows = frame->height;

    if (operation <= 1u) {
        replacement = operation == 0u
                ? NAME_AREA_PALETTE_BASE
                : NAME_AREA_PALETTE_LAST;
        do {
            columns = (cb_u8)frame->width;
            do {
                palette_index = (cb_u8)(*pixel ^ NAME_AREA_PALETTE_BASE);
                if (palette_index <= 0x0fu) {
                    *pixel = replacement;
                }
                ++pixel;
            } while (--columns != 0u);
            pixel += row_skip;
        } while (--rows != 0u);
    } else if (operation == 2u) {
        do {
            columns = (cb_u8)frame->width;
            do {
                palette_index = (cb_u8)(*pixel ^ NAME_AREA_PALETTE_BASE);
                if (palette_index < 0x0fu && palette_index != 0x0eu) {
                    *pixel = (cb_u8)(NAME_AREA_PALETTE_BASE
                            + ((palette_index + 2u) & 0x0fu));
                }
                ++pixel;
            } while (--columns != 0u);
            pixel += row_skip;
        } while (--rows != 0u);
    } else {
        do {
            columns = (cb_u8)frame->width;
            do {
                palette_index = (cb_u8)(*pixel ^ NAME_AREA_PALETTE_BASE);
                if (palette_index <= 0x0fu) {
                    if (palette_index != 0u) {
                        --palette_index;
                    }
                    *pixel = (cb_u8)(NAME_AREA_PALETTE_BASE + palette_index);
                }
                ++pixel;
            } while (--columns != 0u);
            pixel += row_skip;
        } while (--rows != 0u);
    }
}
