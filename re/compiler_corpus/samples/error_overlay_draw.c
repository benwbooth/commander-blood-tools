/* Codegen probe for BLOODPRG 0x000D75. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#define FAR far
#define NEAR near

#define ERROR_CODING 0u
#define ERROR_FILE 1u
#define ERROR_ALLOCATION 2u
#define ERROR_COLOR 15u
#define ROW_HEIGHT 6u
#define CHARACTER_PITCH 4u

extern volatile u16 display_buffer_segment_probe;
extern const u8 coding_text_probe[];
extern const u8 file_text_probe[];
extern const u8 allocation_text_probe[];
extern const u8 handle_text_probe[];
extern const u8 free_text_probe[];
extern char number_buffer_probe[];
extern volatile u16 current_handle_probe;
extern volatile u32 free_bytes_probe;

extern u16 FAR strlen_probe(const u8 FAR *text);
extern u32 FAR layout_probe(u16 columns, u16 rows);
extern void FAR small_text_probe(
        const u8 FAR *text, u16 x, u16 y, u8 color);
extern void FAR decimal_i16_probe(i16 value, char FAR *destination);
extern void FAR decimal_i32_probe(i32 value, char FAR *destination);

void FAR error_overlay_draw_probe(u16 mode, const u8 FAR *detail)
{
    u32 layout;
    u16 saved_display_segment;
    u16 numeric_x;
    u16 x;
    u16 y;

    saved_display_segment = display_buffer_segment_probe;
    display_buffer_segment_probe = 0xa000u;

    if (mode == ERROR_CODING) {
        layout = layout_probe(strlen_probe(coding_text_probe), 1u);
        x = (u16)layout;
        y = (u16)(layout >> 16);
        small_text_probe(coding_text_probe, x, y, ERROR_COLOR);
    } else if (mode == ERROR_FILE) {
        layout = layout_probe(strlen_probe(file_text_probe), 2u);
        x = (u16)layout;
        y = (u16)(layout >> 16);
        small_text_probe(file_text_probe, x, y, ERROR_COLOR);
        small_text_probe(detail, x, (u16)(y + ROW_HEIGHT), ERROR_COLOR);
    } else if (mode == ERROR_ALLOCATION) {
        layout = layout_probe(strlen_probe(allocation_text_probe), 3u);
        x = (u16)layout;
        y = (u16)(layout >> 16);
        small_text_probe(allocation_text_probe, x, y, ERROR_COLOR);

        y = (u16)(y + ROW_HEIGHT);
        small_text_probe(handle_text_probe, x, y, ERROR_COLOR);
        numeric_x = (u16)(x
                + strlen_probe(handle_text_probe) * CHARACTER_PITCH);
        decimal_i16_probe((i16)current_handle_probe, number_buffer_probe);
        small_text_probe(
                (const u8 NEAR *)number_buffer_probe,
                numeric_x,
                y,
                ERROR_COLOR);

        y = (u16)(y + ROW_HEIGHT);
        small_text_probe(free_text_probe, x, y, ERROR_COLOR);
        decimal_i32_probe((i32)free_bytes_probe, number_buffer_probe);
        small_text_probe(
                (const u8 NEAR *)number_buffer_probe,
                numeric_x,
                y,
                ERROR_COLOR);
    }

    display_buffer_segment_probe = saved_display_segment;
}
