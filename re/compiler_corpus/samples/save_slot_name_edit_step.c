/* Codegen probe for BLOODPRG 0x001DD8. */

#include <string.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

extern volatile u8 input_dispatch_state_probe;
extern volatile u16 save_slot_name_length_probe;
extern volatile u16 save_slot_selected_index_probe;
extern volatile u8 NEAR * volatile save_slot_active_name_probe;
extern volatile u8 save_slot_edit_buffer_probe[16];
extern volatile u16 save_slot_row_x_probe;
extern volatile u16 save_slot_row_width_probe;

void FAR framebuffer_rect_fill_probe(
        u8 color, u16 x, u16 y, u16 width, u16 height);
void FAR square_caps_text_draw_display_probe(
        const u8 FAR *text, u16 x, u16 y, u8 color);

#pragma aux framebuffer_rect_fill_probe \
        parm caller [ax] [bx] [cx] [dx] modify exact []
#pragma aux square_caps_text_draw_display_probe \
        parm [ds si] [bx] [dx] [ax] modify exact []

int NEAR save_slot_name_edit_step_probe(void)
{
    u16 name_length;
    u16 row_y;
    u8 key;

    key = input_dispatch_state_probe;
    if (key != 0u) {
        name_length = save_slot_name_length_probe;
        if (key == 0x0du) {
            if (name_length != 0u) {
                memcpy(
                        (void NEAR *)save_slot_active_name_probe,
                        (const void NEAR *)save_slot_edit_buffer_probe,
                        16u);
                return 1;
            }
        } else if ((key >= (u8)'0' && key <= (u8)'9')
                || (key >= (u8)'a' && key <= (u8)'z')) {
            if ((u8)name_length != 14u) {
                save_slot_edit_buffer_probe[name_length] = key;
            }
        } else if (key == 8u && name_length != 0u) {
            --name_length;
            save_slot_edit_buffer_probe[name_length] = (u8)' ';
        }
    }

    row_y = (u16)((u8)save_slot_selected_index_probe * 11u + 39u);
    framebuffer_rect_fill_probe(
            0xe8u,
            save_slot_row_x_probe,
            row_y,
            save_slot_row_width_probe,
            10u);
    square_caps_text_draw_display_probe(
            (const u8 FAR *)save_slot_edit_buffer_probe,
            (u16)(save_slot_row_x_probe + 10u),
            (u16)(row_y + 1u),
            0xefu);
    return 0;
}
