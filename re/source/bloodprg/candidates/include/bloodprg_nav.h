#ifndef BLOODPRG_NAV_H
#define BLOODPRG_NAV_H

#include "bloodprg_common.h"
#include "bloodprg_entity.h"
#include "bloodprg_input.h"
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

extern volatile cb_u8 nav_choice_phase;       /* DS:0x2565 */
extern volatile cb_u16 nav_choice_honk_record; /* DS:0x6754 */
extern volatile cb_u16 nav_choice_radio_record; /* DS:0x6756 */
extern volatile cb_u16 nav_pending_record_link; /* DS:0x675A */
/* SS:0x2B13 in the binary; runtime SS=DS makes this ordinary near data. */
extern volatile cb_u16 nav_kind2_target_offsets[];
extern volatile cb_u16 nav_deferred_record_type; /* DS:0x6768 */
extern volatile cb_u16 nav_deferred_record_link; /* DS:0x676A */
extern volatile char nav_radio_snd_path[];    /* DS:0x0D16 */
extern volatile cb_u8 nav_presentation_reverse; /* DS:0x27E4 */
extern volatile cb_u16 nav_actor_presentation_state; /* DS:0x0A32 */
extern volatile cb_u16 nav_actor_ship_depth_offset; /* DS:0x2527 */
extern volatile cb_i16 nav_actor_zoom_counter; /* DS:0x2B93 */
extern volatile cb_u8 nav_actor_completion_latch; /* DS:0x27E5 */
extern volatile cb_u16 nav_target_presentation_state; /* DS:0x0A34 */
extern volatile cb_u8 nav_actor_transition_phase; /* DS:0x2792 */
extern volatile cb_u16 nav_kind100_target_record; /* DS:0x27D5 */
extern volatile cb_u8 nav_camera_view_active; /* DS:0x278A */
extern volatile cb_u8 nav_camera_view_state; /* DS:0x278B */
extern volatile cb_u8 nav_location_panel_active; /* DS:0x278C */
extern volatile cb_u8 nav_actor_5_active; /* DS:0x278E */
extern volatile cb_u16 nav_selected_location_record; /* DS:0x27BF */
extern volatile cb_u8 nav_screen_rebuild_pending; /* DS:0x27D9 */
extern volatile cb_u8 nav_transition_pending; /* DS:0x27DA */
extern volatile cb_u8 nav_actor_0_busy; /* DS:0x2A7B */
extern volatile cb_u8 nav_actor_1_busy; /* DS:0x2A93 */
extern cb_u32 nav_actor_live_palette_dwords[0x90]; /* DS:0x5251 */
/* The shipped dispatcher keeps ES equal to DS for this destination. */
extern cb_u32 nav_actor_bridge_palette_dwords[0x90]; /* ES:0x5B58 */
extern volatile cb_u8 presentation_mode_flag_27e0; /* DS:0x27E0 */
extern volatile cb_u8 presentation_mode_flag_27e1; /* DS:0x27E1 */
extern volatile cb_u8 presentation_choice_active; /* DS:0x259B */
extern volatile cb_u8 presentation_choice_phase; /* DS:0x259C */
extern const cb_u16 presentation_choice_items[]; /* DS:0x259D */
extern const cb_i16 presentation_choice_target_rect[4]; /* DS:0x25CF */
extern const cb_i16 presentation_choice_current_rect[4]; /* DS:0x2AAB */
extern volatile cb_u16 presentation_choice_result; /* DS:0x0ACA */
extern volatile cb_u8 presentation_list_editing; /* DS:0x27E6 */
extern volatile cb_u16 confirm_dialog_state; /* DS:0x0A32 */
extern const cb_u8 confirm_dialog_question[]; /* DS:0x017B */
extern const cb_u8 confirm_dialog_yes[]; /* DS:0x0189 */
extern const cb_u8 confirm_dialog_no[]; /* DS:0x018D */
extern const bloodprg_rect_i16 confirm_dialog_yes_region; /* DS:0x2555 */
extern const bloodprg_rect_i16 confirm_dialog_no_region; /* DS:0x255D */
extern volatile cb_u8 CB_FAR *nav_presentation_resource_buffer; /* DS:0x0A80 */
extern volatile char CB_FAR fs_presentation_resource_names[][16]; /* FS:0x0C04 */

#if defined(__WATCOMC__)
#pragma aux list_widget_layout_unified parm [si] value [ax]
#pragma aux nav_choice_handler_0 modify exact [ax]
#pragma aux nav_choice_handler_3 modify exact [ax si]
#pragma aux nav_kind2_target_list_build value [ax] modify exact [ax cx]
#endif

int CB_NEAR presentation_line_helper(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x007E1C */
void CB_NEAR nav_actor_handler_1(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x007EC0 */
void CB_NEAR nav_actor_handler_0(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x007F9C */
void CB_NEAR nav_actor_handler_2(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x00813A */
void CB_NEAR nav_actor_handler_3(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x00817E */
void CB_NEAR nav_actor_handler_4(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x0081FB */
void CB_NEAR nav_actor_handler_5(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x008082 */
cb_i16 CB_FAR list_widget_layout_unified(
        const cb_u16 CB_NEAR *items); /* 0x008428 */
void CB_NEAR presentation_choice_transition_step(void); /* 0x001AD3 */
void CB_NEAR confirm_dialog_step(void); /* 0x0014CA */
void CB_NEAR nav_choice_handler_0(void); /* 0x008713 */
void CB_NEAR nav_choice_handler_3(void); /* 0x008848 */
cb_u16 CB_FAR nav_kind2_target_list_build(void); /* 0x0071CF */

#endif
