#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_vm.h"

#define DLG_MENU_LEFT 10u
#define DLG_MENU_TOP 8u
#define DLG_MENU_ROW_HEIGHT 8u
#define DLG_MENU_WORD_GAP 6u
#define DLG_MENU_RIGHT 300
#define DLG_MENU_COLOR 0xefu
#define DLG_MENU_OWNER_OFFSET 0x67b0u

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define DLG_MENU_CURRENT_WORD(dictionary, offset) \
    ((const cb_u8 CB_FAR *)MK_FP( \
        FP_SEG(dictionary), \
        (cb_u16)(FP_OFF(dictionary) + (offset))))
#define DLG_MENU_NEXT_WORD(dictionary, offset) \
    ((const cb_u8 CB_FAR *)MK_FP(FP_SEG(dictionary), (offset)))
#define DLG_MENU_CURSOR_OFFSET(cursor) ((cb_u16)FP_OFF(cursor))
#else
#define DLG_MENU_CURRENT_WORD(dictionary, offset) ((dictionary) + (offset))
#define DLG_MENU_NEXT_WORD(dictionary, offset) ((dictionary) + (offset))
#define DLG_MENU_CURSOR_OFFSET(cursor) ((cb_u16)(cursor))
#endif

void CB_FAR dlg_menu_words_inline_reveal_step(void)
{
    const cb_u16 CB_FAR *menu_cursor;
    const cb_u8 CB_FAR *dictionary;
    const cb_u8 CB_FAR *word;
    const cb_u8 CB_FAR *next_word;
    cb_u16 word_offset;
    cb_u16 next_offset;
    cb_u16 draw_width;
    cb_u16 next_width;
    cb_u16 next_x;
    cb_u16 completion_hold;
    cb_u16 y;
    cb_u8 next_character;

    if ((vm_presentation_defer_a & 1u) == 0u) {
        if ((vm_presentation_hold_ready & 1u) == 0u ||
                vm_presentation_owner_offset != DLG_MENU_OWNER_OFFSET) {
            return;
        }
    }

    vm_text_menu_inline_x = DLG_MENU_LEFT;
    menu_cursor = vm_text_menu_words;
    dictionary = (const cb_u8 CB_FAR *)vm_dic_words;
    y = DLG_MENU_TOP;

    for (;;) {
        word_offset = *menu_cursor;
        if (word_offset == 0u || word_offset == 0xffffu) {
            break;
        }

        word = DLG_MENU_CURRENT_WORD(dictionary, word_offset);
        planar_dialogue_text_render(
            word, vm_text_menu_inline_x, y, DLG_MENU_COLOR);
        draw_width = main_font_draw_width;

        ++menu_cursor;
        next_offset = *menu_cursor;
        next_word = DLG_MENU_NEXT_WORD(dictionary, next_offset);
        next_character = *next_word;

        if (next_character == '.' || next_character == ',' ||
                next_character == ':' || next_character == '!' ||
                next_character == '?') {
            vm_text_menu_inline_x =
                (cb_u16)(vm_text_menu_inline_x + draw_width);
        } else {
            next_x = (cb_u16)(
                vm_text_menu_inline_x + draw_width + DLG_MENU_WORD_GAP);
            vm_text_menu_inline_x = next_x;
            next_width = text_width_dual_font_far(next_word, 1);
            if ((cb_i16)(cb_u16)(next_x + next_width) >=
                    DLG_MENU_RIGHT) {
                vm_text_menu_inline_x = DLG_MENU_LEFT;
                y = (cb_u16)(y + DLG_MENU_ROW_HEIGHT);
            }
        }

        if (DLG_MENU_CURSOR_OFFSET(menu_cursor) >= vm_text_menu_end) {
            if (vm_dialogue_hold_countdown == 0u) {
                vm_text_menu_end = (cb_u16)(vm_text_menu_end + 2u);
                vm_dialogue_hold_countdown = presentation_choice_result;
            }
            return;
        }
    }

    if ((vm_presentation_hold_ready & 1u) == 0u &&
            (vm_dialogue_hold_complete & 1u) == 0u) {
        completion_hold = (cb_u16)(
            vm_operand_word_count * (presentation_choice_result >> 1) +
            DLG_MENU_WORD_GAP);
        vm_dialogue_hold_countdown = completion_hold;
        vm_dialogue_hold_complete = 1u;
    }

}
