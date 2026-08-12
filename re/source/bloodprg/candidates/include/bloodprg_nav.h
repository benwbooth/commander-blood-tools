#ifndef BLOODPRG_NAV_H
#define BLOODPRG_NAV_H

#include "bloodprg_common.h"
#include "bloodprg_entity.h"
#include "bloodprg_resource.h"
#include "bloodprg_vm.h"

#define BLOODPRG_PRESENTATION_LINE_LOADED_FLAG 0x04u
#define BLOODPRG_PRESENTATION_UI_BUSY_GATE 0x08u
#define BLOODPRG_PRESENTATION_UI_REDRAW_FLAG 0x04u

typedef struct bloodprg_presentation_resource_header {
    cb_u16 field_00;
    cb_u16 terminal_frame;
} bloodprg_presentation_resource_header;

typedef struct bloodprg_presentation_line_record {
    cb_u8 flags;
    cb_u8 pad_01;
    cb_u16 resource_id;
    cb_u16 pad_04;
    cb_u16 terminal_frame;
    cb_u16 frame_index;
    cb_u8 pad_0a[10];
    cb_u16 draw_x;
    cb_u16 draw_y;
} bloodprg_presentation_line_record;

extern volatile cb_u8 nav_choice_phase;       /* GS:0x2565 */
extern volatile cb_u16 nav_choice_honk_record; /* GS:0x6754 */
extern volatile cb_u16 nav_choice_radio_record; /* GS:0x6756 */
extern volatile cb_u16 nav_deferred_record_type; /* GS:0x6768 */
extern volatile cb_u16 nav_deferred_record_link; /* GS:0x676A */
extern volatile char nav_radio_snd_path[];    /* GS:0x0D16 */
extern volatile cb_u8 nav_presentation_reverse; /* DS:0x27E4 */
extern volatile cb_u8 presentation_mode_flag_27e0; /* DS:0x27E0 */
extern volatile cb_u8 presentation_mode_flag_27e1; /* DS:0x27E1 */
extern volatile cb_u8 CB_FAR *nav_presentation_resource_buffer; /* DS:0x0A80 */
extern const volatile char fs_presentation_resource_names[][16]; /* FS:0x0C04 */

int CB_NEAR presentation_line_helper(
        volatile bloodprg_presentation_line_record *line); /* 0x007E1C */

#endif
