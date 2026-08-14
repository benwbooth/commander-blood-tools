#include <string.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_save.h"

#define SAVE_SLOT_KEY_ENTER 0x0du
#define SAVE_SLOT_KEY_BACKSPACE 0x08u
#define SAVE_SLOT_ROW_HEIGHT 10u
#define SAVE_SLOT_ROW_PITCH 11u
#define SAVE_SLOT_ROW_TOP 39u
#define SAVE_SLOT_TEXT_INSET 10u
#define SAVE_SLOT_BACKGROUND_COLOR 0xe8u
#define SAVE_SLOT_TEXT_COLOR 0xefu

int CB_NEAR save_slot_name_edit_step(void)
{
    cb_u16 name_length;
    cb_u16 row_y;
    cb_u8 key;

    key = input_dispatch_state_b15;
    if (key != 0u) {
        name_length = save_slot_name_length;
        if (key == SAVE_SLOT_KEY_ENTER) {
            if (name_length != 0u) {
                memcpy(
                        (void CB_NEAR *)save_slot_active_name,
                        (const void CB_NEAR *)save_slot_edit_buffer,
                        BLOODPRG_SAVE_SLOT_NAME_BYTES);
                return 1;
            }
        } else if ((key >= (cb_u8)'0' && key <= (cb_u8)'9')
                || (key >= (cb_u8)'a' && key <= (cb_u8)'z')) {
            if ((cb_u8)name_length != BLOODPRG_SAVE_SLOT_NAME_LIMIT) {
                save_slot_edit_buffer[name_length] = key;
            }
        } else if (key == SAVE_SLOT_KEY_BACKSPACE && name_length != 0u) {
            --name_length;
            save_slot_edit_buffer[name_length] = (cb_u8)' ';
        }
    }

    row_y = (cb_u16)((cb_u8)save_slot_selected_index
            * SAVE_SLOT_ROW_PITCH + SAVE_SLOT_ROW_TOP);
    framebuffer_rect_fill(
            SAVE_SLOT_BACKGROUND_COLOR,
            save_slot_row_x,
            row_y,
            save_slot_row_width,
            SAVE_SLOT_ROW_HEIGHT);
    square_caps_text_draw_display(
            (const cb_u8 CB_FAR *)save_slot_edit_buffer,
            (cb_u16)(save_slot_row_x + SAVE_SLOT_TEXT_INSET),
            (cb_u16)(row_y + 1u),
            SAVE_SLOT_TEXT_COLOR);
    return 0;
}
