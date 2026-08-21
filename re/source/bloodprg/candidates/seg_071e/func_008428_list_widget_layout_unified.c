#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_ship3d.h"

#define LIST_WIDGET_END 0xffffu
#define LIST_WIDGET_DEFAULT_WIDTH 100u
#define LIST_WIDGET_EXTRA_WIDTH 55u
#define LIST_WIDGET_EXTRA_HEIGHT 10u
#define LIST_WIDGET_ROW_PITCH 11u
#define LIST_WIDGET_WIDTH_PADDING 20u
#define LIST_WIDGET_HEIGHT_PADDING 8u
#define LIST_WIDGET_SCREEN_HEIGHT 200u
#define LIST_WIDGET_TEXT_X_INSET 10u
#define LIST_WIDGET_TEXT_Y_INSET 4u
#define LIST_WIDGET_IDLE_STATE 1u
#define LIST_WIDGET_HOVER_STATE 6u
#define LIST_WIDGET_ACTIVE_STATE 7u
#define LIST_WIDGET_DEFAULT_COLOR 0xe8u
#define LIST_WIDGET_HOVER_COLOR 0xefu
#define LIST_WIDGET_ACTIVE_COLOR 0xfeu
#define LIST_WIDGET_ACTIVE_NAME_OFFSET 0x2734u
#define LIST_WIDGET_EDIT_BUFFER_OFFSET 0x273bu

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define LIST_WIDGET_WORD_AT(segment, offset) \
    ((const volatile cb_u16 CB_FAR *)MK_FP((segment), (offset)))
#define LIST_WIDGET_TEXT_AT(segment, offset) \
    ((const cb_u8 CB_FAR *)MK_FP((segment), (offset)))
typedef cb_u16 list_widget_segment;
#else
#define LIST_WIDGET_WORD_AT(segment, offset) \
    ((const volatile cb_u16 CB_FAR *)(cb_u16)(offset))
#define LIST_WIDGET_TEXT_AT(segment, offset) \
    ((const cb_u8 CB_FAR *)(cb_u16)(offset))
typedef cb_u16 list_widget_segment;
#endif

#if defined(__WATCOMC__)
static list_widget_segment CB_NEAR list_widget_inherited_es(void);
#pragma aux list_widget_inherited_es = \
        "mov ax,es" \
        value [ax] modify exact []

#elif defined(__TURBOC__) || defined(__BORLANDC__)
static list_widget_segment CB_NEAR list_widget_inherited_es(void)
{
    asm mov ax, es;
}
#else
static list_widget_segment CB_NEAR list_widget_inherited_es(void)
{
    return 0u;
}
#endif

cb_i16 CB_FAR list_widget_layout_unified(
        const cb_u16 CB_NEAR *items)
{
    const cb_u16 CB_NEAR *item;
    volatile bloodprg_rect_i16 CB_NEAR *layout;
    list_widget_segment label_segment;
    cb_u16 item_offset;
    cb_u16 width_count;
    cb_u16 fill_count;
    cb_u16 max_width;
    cb_u16 row_extent;
    cb_u16 layout_width;
    cb_u16 layout_height;
    cb_u16 content_width;
    cb_u16 row_offset;
    cb_u16 row_x;
    cb_u16 row_y;
    cb_u16 index;
    cb_u8 color;

    label_segment = list_widget_inherited_es();
    nav_target_selection = 0u;
    row_extent = 0u;
    max_width = LIST_WIDGET_DEFAULT_WIDTH;
    if ((ship_3d_target_layout_extra_entry & 1u) != 0u) {
        row_extent = LIST_WIDGET_EXTRA_HEIGHT;
        max_width = LIST_WIDGET_EXTRA_WIDTH;
    }

    item = items;
    width_count = 0u;
    for (;;) {
        item_offset = *item++;
        if (item_offset == 0u || item_offset == LIST_WIDGET_END) {
            break;
        }

        if (item_offset == *LIST_WIDGET_WORD_AT(
                label_segment, LIST_WIDGET_ACTIVE_NAME_OFFSET)) {
            item_offset = LIST_WIDGET_EDIT_BUFFER_OFFSET;
        }
        list_widget_label_widths[width_count] = text_width_dual_font(
                LIST_WIDGET_TEXT_AT(label_segment, item_offset), 0);
        if (list_widget_label_widths[width_count] >= max_width) {
            max_width = list_widget_label_widths[width_count];
        }
        ++width_count;
        row_extent = (cb_u16)(row_extent + LIST_WIDGET_ROW_PITCH);
    }

    if ((ship_3d_target_layout_extra_entry & 1u) != 0u) {
        list_widget_label_widths[width_count++] = LIST_WIDGET_EXTRA_WIDTH;
    }
    if ((ship_3d_target_layout_preserve_widths & 1u) == 0u) {
        /* The original feeds the byte distance to REP STOSW, doubling the
         * logical entry count. Preserve that observable overwrite. */
        fill_count = (cb_u16)(width_count << 1);
        for (index = 0u; index < fill_count; ++index) {
            list_widget_label_widths[index] = max_width;
        }
    }

    layout = (volatile bloodprg_rect_i16 CB_NEAR *)
            presentation_choice_current_rect;
    layout_width = (cb_u16)(max_width + LIST_WIDGET_WIDTH_PADDING);
    layout_height = (cb_u16)(row_extent + LIST_WIDGET_HEIGHT_PADDING);
    layout->width = (cb_i16)layout_width;
    layout->height = (cb_i16)layout_height;
    layout->x = (cb_i16)(ship_3d_target_layout_center_x
            - (layout_width >> 1));
    layout->y = (cb_i16)((cb_u16)(LIST_WIDGET_SCREEN_HEIGHT
            - layout_height) >> 1);

    if ((presentation_list_editing & 1u) != 0u) {
        return -1;
    }

    framebuffer_rect_palette_remap(
            framebuffer_transition_remap_table,
            (cb_u16)layout->x,
            (cb_u16)layout->y,
            layout_width,
            layout_height);

    nav_target_hover_row = 0u;
    row_y = (cb_u16)((cb_u16)layout->y + LIST_WIDGET_TEXT_Y_INSET);
    if (mouse_x >= layout->x
            && mouse_x <= (cb_i16)((cb_u16)layout->x + layout_width)) {
        row_offset = (cb_u16)((cb_u16)mouse_y - row_y);
        if ((cb_i16)row_offset >= 0
                && (cb_i16)row_offset
                    < (cb_i16)(layout_height - LIST_WIDGET_HEIGHT_PADDING)) {
            nav_target_hover_row = (cb_u8)(
                    row_offset / LIST_WIDGET_ROW_PITCH + 1u);
            if (nav_target_presentation_state != LIST_WIDGET_HOVER_STATE) {
                nav_target_presentation_state = 0u;
                nav_actor_presentation_state = LIST_WIDGET_HOVER_STATE;
            }
            if ((mouse_primary_pressed & 1u) != 0u) {
                nav_actor_presentation_state = LIST_WIDGET_ACTIVE_STATE;
                nav_target_selection = nav_target_hover_row;
                snd_play_clip(0);
            }
        } else if (nav_target_presentation_state != LIST_WIDGET_IDLE_STATE) {
            nav_target_presentation_state = 0u;
            nav_actor_presentation_state = LIST_WIDGET_IDLE_STATE;
        }
    } else if (nav_target_presentation_state != LIST_WIDGET_IDLE_STATE) {
        nav_target_presentation_state = 0u;
        nav_actor_presentation_state = LIST_WIDGET_IDLE_STATE;
    }

    content_width = (cb_u16)(layout_width - LIST_WIDGET_WIDTH_PADDING);
    row_x = (cb_u16)((cb_u16)layout->x + LIST_WIDGET_TEXT_X_INSET);
    item = items;
    index = 0u;
    for (;;) {
        item_offset = *item++;
        if (item_offset == 0u || item_offset == LIST_WIDGET_END) {
            break;
        }

        color = LIST_WIDGET_DEFAULT_COLOR;
        --nav_target_hover_row;
        if (nav_target_hover_row == 0u) {
            color = LIST_WIDGET_HOVER_COLOR;
            if ((mouse_primary_pressed & 1u) != 0u) {
                color = LIST_WIDGET_ACTIVE_COLOR;
            }
        }
        if (item_offset == *LIST_WIDGET_WORD_AT(
                label_segment, LIST_WIDGET_ACTIVE_NAME_OFFSET)) {
            item_offset = LIST_WIDGET_EDIT_BUFFER_OFFSET;
        }
        square_caps_text_draw_display(
                LIST_WIDGET_TEXT_AT(label_segment, item_offset),
                (cb_u16)(row_x
                    + ((cb_u16)(content_width
                        - list_widget_label_widths[index]) >> 1)),
                row_y,
                color);
        ++index;
        row_y = (cb_u16)(row_y + LIST_WIDGET_ROW_PITCH);
    }

    if ((ship_3d_target_layout_extra_entry & 1u) != 0u) {
        color = LIST_WIDGET_DEFAULT_COLOR;
        --nav_target_hover_row;
        if (nav_target_hover_row == 0u) {
            color = LIST_WIDGET_HOVER_COLOR;
            if ((mouse_primary_pressed & 1u) != 0u) {
                color = LIST_WIDGET_ACTIVE_COLOR;
            }
        }
        square_caps_text_draw_display(
                list_widget_cancel_label,
                (cb_u16)(row_x
                    + ((cb_u16)(content_width
                        - list_widget_label_widths[index]) >> 1)),
                row_y,
                color);
    }

    return (cb_i16)(cb_i8)(nav_target_selection - 1u);
}
