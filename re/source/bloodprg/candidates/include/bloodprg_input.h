#ifndef BLOODPRG_INPUT_H
#define BLOODPRG_INPUT_H

#include "bloodprg_common.h"

#define BLOODPRG_MOUSE_BUTTON_PRIMARY 0x01u
#define BLOODPRG_MOUSE_BUTTON_SECONDARY 0x02u

extern volatile cb_u16 mouse_button_state;          /* DS:0x0A2E */
extern volatile cb_u16 mouse_previous_button_state; /* DS:0x0A30 */
extern volatile cb_u8 mouse_primary_pressed;        /* DS:0x0A3E */
extern volatile cb_u8 mouse_secondary_pressed;      /* DS:0x0A3F */
extern volatile cb_u8 mouse_press_pending;           /* DS:0x0A40 */

void CB_NEAR mouse_button_edges_update(void); /* 0x001FBC */

#endif
