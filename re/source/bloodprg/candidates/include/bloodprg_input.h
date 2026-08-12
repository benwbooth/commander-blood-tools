#ifndef BLOODPRG_INPUT_H
#define BLOODPRG_INPUT_H

#include "bloodprg_common.h"

#define BLOODPRG_MOUSE_BUTTON_PRIMARY 0x01u
#define BLOODPRG_MOUSE_BUTTON_SECONDARY 0x02u
#define BLOODPRG_UI_HIT_FLAG 0x08u

typedef struct bloodprg_rect_i16 {
    cb_i16 x;
    cb_i16 y;
    cb_i16 width;
    cb_i16 height;
} bloodprg_rect_i16;

extern volatile cb_i16 mouse_x;                  /* DS:0x0A2A */
extern volatile cb_i16 mouse_y;                  /* DS:0x0A2C */
extern volatile cb_u16 mouse_button_state;          /* DS:0x0A2E */
extern volatile cb_u16 mouse_previous_button_state; /* DS:0x0A30 */
extern volatile cb_u8 mouse_primary_pressed;        /* DS:0x0A3E */
extern volatile cb_u8 mouse_secondary_pressed;      /* DS:0x0A3F */
extern volatile cb_u8 mouse_press_pending;           /* DS:0x0A40 */

void CB_NEAR mouse_button_edges_update(void); /* 0x001FBC */
void CB_NEAR mouse_hit_test(const bloodprg_rect_i16 CB_NEAR *rect,
        volatile cb_u8 CB_NEAR *flags); /* 0x008269 */
int CB_FAR region_record_hittest(
        const bloodprg_rect_i16 CB_NEAR *rect); /* 0x008295 */

#endif
