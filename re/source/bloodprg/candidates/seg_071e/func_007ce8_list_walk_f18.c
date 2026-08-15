#include <dos.h>

#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_graphics.h"

#define CENTERED_TEXT_WRAP_COLUMN 0x1Cu
#define CENTERED_TEXT_MIDDLE_X 0x00A0u
#define CENTERED_TEXT_GLYPH_HALF_WIDTH 4u
#define CENTERED_TEXT_FIRST_Y 0x006Eu
#define CENTERED_TEXT_LINE_HEIGHT 8u
#define CENTERED_TEXT_COLOR 0xEFu

void CB_NEAR list_walk_f18(void)
{
    const cb_u8 CB_FAR *draw_text;
    cb_i16 threshold;
    cb_i16 next_threshold;
    cb_u16 entry_offset;
    cb_u16 scan_offset;
    cb_u16 text_offset;
    cb_u16 line_count;
    cb_u16 line_index;
    cb_u16 line_length;
    cb_u16 line_y;
    cb_u8 character;

    entry_offset = (cb_u16)byte_parser_stream_0f18_cursor;
    threshold = (cb_i16)((cb_u16)byte_parser_stream_segment[entry_offset]
            | ((cb_u16)byte_parser_stream_segment[
                    (cb_u16)(entry_offset + 1u)] << 8));
    if (threshold < 0 || threshold > byte_parser_table_131c_visible_index) {
        return;
    }

    scan_offset = (cb_u16)(entry_offset + sizeof(cb_u16));
    text_offset = scan_offset;
    line_count = 0u;
    line_length = 0u;
    for (;;) {
        character = byte_parser_stream_segment[scan_offset++];
        ++line_length;
        if (character == 0u) {
            --line_length;
            break;
        }
        if (character == ' '
                && (cb_i8)(cb_u8)line_length
                        >= (cb_i8)CENTERED_TEXT_WRAP_COLUMN) {
            centered_text_line_layout[line_count].character_count =
                    line_length;
            centered_text_line_layout[line_count].centered_x =
                    (cb_u16)(CENTERED_TEXT_MIDDLE_X
                    - (cb_u16)(line_length * CENTERED_TEXT_GLYPH_HALF_WIDTH));
            ++line_count;
            line_length = 0u;
        }
    }

    centered_text_line_layout[line_count].character_count = line_length;
    centered_text_line_layout[line_count].centered_x =
            (cb_u16)(CENTERED_TEXT_MIDDLE_X
            - (cb_u16)(line_length * CENTERED_TEXT_GLYPH_HALF_WIDTH));
    ++line_count;

    draw_text = (const cb_u8 CB_FAR *)MK_FP(
            FP_SEG((const void CB_FAR *)&byte_parser_stream_0f18_cursor),
            text_offset);
    line_index = 0u;
    line_y = CENTERED_TEXT_FIRST_Y;
    do {
        draw_text = font8x8_text_draw_display(
                draw_text,
                centered_text_line_layout[line_index].centered_x,
                line_y,
                (cb_u16)(
                        (centered_text_line_layout[line_index].character_count
                                << 8)
                        | CENTERED_TEXT_COLOR));
        line_y = (cb_u16)(line_y + CENTERED_TEXT_LINE_HEIGHT);
        ++line_index;
    } while (line_index != line_count);

    next_threshold = (cb_i16)((cb_u16)byte_parser_stream_segment[scan_offset]
            | ((cb_u16)byte_parser_stream_segment[
                    (cb_u16)(scan_offset + 1u)] << 8));
    if (next_threshold >= 0
            && next_threshold <= byte_parser_table_131c_visible_index) {
        byte_parser_stream_0f18_cursor = (cb_game_char_ptr)scan_offset;
    }
}
