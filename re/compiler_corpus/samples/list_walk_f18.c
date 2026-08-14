#include <dos.h>

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned short u16;
typedef signed short i16;

#define NEAR __near
#define FAR __far
#define GAME_DATA __based(__segname("GAME_DATA"))

#define CENTERED_TEXT_WRAP_COLUMN 0x1Cu
#define CENTERED_TEXT_MIDDLE_X 0x00A0u
#define CENTERED_TEXT_GLYPH_HALF_WIDTH 4u
#define CENTERED_TEXT_FIRST_Y 0x006Eu
#define CENTERED_TEXT_LINE_HEIGHT 8u
#define CENTERED_TEXT_COLOR 0xEFu

typedef volatile char GAME_DATA *game_char_ptr;

typedef struct centered_text_line {
    u16 character_count;
    u16 centered_x;
} centered_text_line;

extern volatile game_char_ptr GAME_DATA byte_parser_stream_0f18_cursor;
extern volatile i16 GAME_DATA byte_parser_table_131c_visible_index;
extern volatile u8 GAME_DATA byte_parser_stream_segment[];
extern volatile centered_text_line NEAR centered_text_line_layout[];

const u8 FAR *FAR font8x8_text_draw_display(
        const u8 FAR *text,
        u16 x,
        u16 y,
        u16 color_and_limit);

#pragma aux font8x8_text_draw_display \
        parm [ds si] [ax] [bx] [dx] value [ds si] modify exact [si]

void NEAR list_walk_f18(void)
{
    const u8 FAR *draw_text;
    i16 threshold;
    i16 next_threshold;
    u16 entry_offset;
    u16 scan_offset;
    u16 text_offset;
    u16 line_count;
    u16 line_index;
    u16 line_length;
    u8 character;

    entry_offset = (u16)byte_parser_stream_0f18_cursor;
    threshold = *(volatile i16 GAME_DATA *)
            (byte_parser_stream_segment + entry_offset);
    if (threshold < 0 || threshold > byte_parser_table_131c_visible_index) {
        return;
    }

    scan_offset = (u16)(entry_offset + sizeof(u16));
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
                && (i8)(u8)line_length >= (i8)CENTERED_TEXT_WRAP_COLUMN) {
            centered_text_line_layout[line_count].character_count =
                    line_length;
            centered_text_line_layout[line_count].centered_x =
                    (u16)(CENTERED_TEXT_MIDDLE_X
                    - (u16)(line_length * CENTERED_TEXT_GLYPH_HALF_WIDTH));
            ++line_count;
            line_length = 0u;
        }
    }

    centered_text_line_layout[line_count].character_count = line_length;
    centered_text_line_layout[line_count].centered_x =
            (u16)(CENTERED_TEXT_MIDDLE_X
            - (u16)(line_length * CENTERED_TEXT_GLYPH_HALF_WIDTH));
    ++line_count;

    draw_text = (const u8 FAR *)MK_FP(
            FP_SEG((const void FAR *)&byte_parser_stream_0f18_cursor),
            text_offset);
    for (line_index = 0u; line_index < line_count; ++line_index) {
        draw_text = font8x8_text_draw_display(
                draw_text,
                centered_text_line_layout[line_index].centered_x,
                (u16)(CENTERED_TEXT_FIRST_Y
                        + line_index * CENTERED_TEXT_LINE_HEIGHT),
                (u16)((centered_text_line_layout[line_index].character_count
                        << 8) | CENTERED_TEXT_COLOR));
    }

    next_threshold = *(volatile i16 GAME_DATA *)
            (byte_parser_stream_segment + scan_offset);
    if (next_threshold >= 0
            && next_threshold <= byte_parser_table_131c_visible_index) {
        byte_parser_stream_0f18_cursor = (game_char_ptr)scan_offset;
    }
}
