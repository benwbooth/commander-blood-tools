/* Codegen probe for BLOODPRG 0x0072A8. */
typedef unsigned char u8;
typedef signed short i16;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#else
#define FAR
#endif

#define MENU_CURRENT_WORD(dictionary, offset) \
    ((const u8 FAR *)MK_FP( \
        FP_SEG(dictionary), \
        (u16)(FP_OFF(dictionary) + (offset))))
#define MENU_NEXT_WORD(dictionary, offset) \
    ((const u8 FAR *)MK_FP(FP_SEG(dictionary), (offset)))

extern volatile u8 presentation_defer_probe;
extern volatile u8 presentation_ready_probe;
extern volatile u16 presentation_owner_probe;
extern volatile u16 menu_inline_x_probe;
extern const u16 FAR * volatile menu_words_probe;
extern volatile u16 menu_end_probe;
extern const u8 FAR *dictionary_probe;
extern volatile u16 draw_width_probe;
extern volatile u16 hold_countdown_probe;
extern volatile u16 word_count_probe;
extern volatile u16 word_delay_probe;
extern volatile u8 hold_complete_probe;

extern void FAR dialogue_text_draw_probe(
        const u8 FAR *text, u16 x, u16 y, u8 color);
extern u16 FAR text_width_probe(const u8 FAR *text, int use_main_font);

#if defined(__WATCOMC__)
#pragma aux dialogue_text_draw_probe parm [ds si] [bx] [dx] [ax] modify exact []
#pragma aux text_width_probe parm [ds si] [ax] value [ax] modify exact [ax]
#pragma aux menu_words_inline_reveal_step_probe modify exact []
#endif

void FAR menu_words_inline_reveal_step_probe(void)
{
    const u16 FAR *menu_cursor;
    const u8 FAR *dictionary;
    const u8 FAR *word;
    const u8 FAR *next_word;
    u16 word_offset;
    u16 next_offset;
    u16 draw_width;
    u16 next_width;
    u16 next_x;
    u16 completion_hold;
    u16 y;
    u8 next_character;

    if ((presentation_defer_probe & 1u) == 0u) {
        if ((presentation_ready_probe & 1u) == 0u ||
                presentation_owner_probe != 0x67b0u) {
            return;
        }
    }

    menu_inline_x_probe = 10u;
    menu_cursor = menu_words_probe;
    dictionary = dictionary_probe;
    y = 8u;

    for (;;) {
        word_offset = *menu_cursor;
        if (word_offset == 0u || word_offset == 0xffffu) {
            break;
        }

        word = MENU_CURRENT_WORD(dictionary, word_offset);
        dialogue_text_draw_probe(word, menu_inline_x_probe, y, 0xefu);
        draw_width = draw_width_probe;

        ++menu_cursor;
        next_offset = *menu_cursor;
        next_word = MENU_NEXT_WORD(dictionary, next_offset);
        next_character = *next_word;

        if (next_character == '.' || next_character == ',' ||
                next_character == ':' || next_character == '!' ||
                next_character == '?') {
            menu_inline_x_probe = (u16)(menu_inline_x_probe + draw_width);
        } else {
            next_x = (u16)(menu_inline_x_probe + draw_width + 6u);
            menu_inline_x_probe = next_x;
            next_width = text_width_probe(next_word, 1);
            if ((i16)(u16)(next_x + next_width) >= 300) {
                menu_inline_x_probe = 10u;
                y = (u16)(y + 8u);
            }
        }

        if ((u16)FP_OFF(menu_cursor) >= menu_end_probe) {
            if (hold_countdown_probe == 0u) {
                menu_end_probe = (u16)(menu_end_probe + 2u);
                hold_countdown_probe = word_delay_probe;
            }
            return;
        }
    }

    if ((presentation_ready_probe & 1u) == 0u &&
            (hold_complete_probe & 1u) == 0u) {
        completion_hold =
            (u16)(word_count_probe * (word_delay_probe >> 1) + 6u);
        hold_countdown_probe = completion_hold;
        hold_complete_probe = 1u;
    }
}
