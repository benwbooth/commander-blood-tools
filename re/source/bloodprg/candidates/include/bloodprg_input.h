#ifndef BLOODPRG_INPUT_H
#define BLOODPRG_INPUT_H

#include "bloodprg_common.h"

#define BLOODPRG_MOUSE_BUTTON_PRIMARY 0x01u
#define BLOODPRG_MOUSE_BUTTON_SECONDARY 0x02u
#define BLOODPRG_UI_HIT_FLAG 0x08u
#define BLOODPRG_INPUT_ACTION_COUNT 16u

typedef struct bloodprg_rect_i16 {
    cb_i16 x;
    cb_i16 y;
    cb_i16 width;
    cb_i16 height;
} bloodprg_rect_i16;

typedef void (CB_NEAR *bloodprg_input_action_handler)(cb_u8 raw_low_byte);

extern volatile cb_i16 mouse_x;                     /* DS:0x0A2A */
extern volatile cb_i16 mouse_y;                     /* DS:0x0A2C */
extern volatile cb_u16 mouse_button_state;          /* DS:0x0A2E */
extern volatile cb_u16 mouse_previous_button_state; /* DS:0x0A30 */
extern volatile cb_i16 mouse_last_x;                 /* GS:0x0A38 */
extern volatile cb_i16 mouse_last_y;                 /* GS:0x0A3A */
extern volatile cb_u8 mouse_primary_pressed;        /* DS:0x0A3E */
extern volatile cb_u8 mouse_secondary_pressed;      /* DS:0x0A3F */
extern volatile cb_u8 mouse_press_pending;          /* DS:0x0A40 */
extern volatile cb_u16 mouse_motion_idle_counter;   /* GS:0x0B3B */
extern volatile cb_u16 mouse_motion_idle_counter_ds; /* DS:0x0B3B alias */
extern volatile cb_u8 input_dispatch_state_b15;     /* DS:0x0B15 */
extern volatile cb_u16 input_directory_selection_offset; /* GS:0x679E */
extern volatile cb_u8 input_selection_mode_flags;  /* DS/GS:0x67A6 */
extern const cb_i8 CB_CODE_DATA
        input_action_translation[256];              /* CS:0x113E */
extern bloodprg_input_action_handler CB_CODE_DATA
        input_action_handlers[BLOODPRG_INPUT_ACTION_COUNT]; /* CS:0x123E */

void CB_FAR poll_mouse(void); /* 0x000D0E */
cb_u16 CB_NEAR mouse_button_edges_update(void); /* 0x001FBC */
void CB_NEAR mouse_hit_test(const volatile bloodprg_rect_i16 CB_NEAR *rect,
        volatile cb_u8 CB_NEAR *flags); /* 0x008269 */
int CB_FAR region_record_hittest(
        const volatile bloodprg_rect_i16 CB_NEAR *rect); /* 0x008295 */
cb_i16 CB_FAR ui_region_31_poll(void);           /* 0x0082C3 */
void CB_FAR input_action_dispatch(void);        /* 0x00210E */
void CB_NEAR input_action_move_previous(cb_u8 raw_low_byte); /* 0x002140 */
void CB_NEAR input_action_move_next(cb_u8 raw_low_byte);     /* 0x00218D */
void CB_NEAR input_action_noop_2(cb_u8 raw_low_byte);        /* 0x002201 */
void CB_NEAR input_action_noop_3(cb_u8 raw_low_byte);        /* 0x002202 */
void CB_NEAR input_action_request_shutdown(cb_u8 raw_low_byte); /* 0x002203 */
void CB_NEAR input_action_noop_5(cb_u8 raw_low_byte);        /* 0x002209 */
void CB_NEAR input_action_noop_9(cb_u8 raw_low_byte);        /* 0x00220A */
void CB_NEAR input_action_noop_10(cb_u8 raw_low_byte);       /* 0x00220B */
void CB_NEAR input_action_noop_11(cb_u8 raw_low_byte);       /* 0x00220C */
void CB_NEAR input_action_noop_12(cb_u8 raw_low_byte);       /* 0x00220D */
void CB_NEAR input_action_noop_13(cb_u8 raw_low_byte);       /* 0x002216 */
void CB_NEAR input_action_noop_14(cb_u8 raw_low_byte);       /* 0x00221F */
void CB_NEAR input_action_accept(cb_u8 raw_low_byte);        /* 0x002224 */
void CB_NEAR input_action_cancel(cb_u8 raw_low_byte);        /* 0x00224D */
void CB_NEAR input_action_toggle_pause(cb_u8 raw_low_byte);  /* 0x0022B2 */
void CB_NEAR input_action_latch_text_key(cb_u8 raw_low_byte); /* 0x0022D0 */

#if defined(__WATCOMC__)
#pragma aux mouse_button_edges_update value [ax] modify exact [ax]
#pragma aux ui_region_31_poll value [ax] modify exact [ax]
#endif

#endif
