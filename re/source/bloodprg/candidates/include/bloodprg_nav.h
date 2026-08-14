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

typedef struct bloodprg_nav_actor_slot {
    cb_u8 flags;
    cb_u8 reserved_01[9];
    cb_u16 target_arc;
    bloodprg_rect_i16 hit_rect;
    cb_u8 reserved_14[4];
} bloodprg_nav_actor_slot;

typedef void (CB_NEAR *bloodprg_nav_actor_handler)(
        volatile bloodprg_presentation_line_record CB_NEAR *line);
typedef void (CB_NEAR *bloodprg_nav_choice_handler)(void);

typedef char bloodprg_nav_actor_slot_size_must_be_24[
        sizeof(bloodprg_nav_actor_slot) == 24 ? 1 : -1];

extern volatile cb_u8 nav_choice_phase;       /* DS:0x2565 */
extern volatile cb_u16 nav_choice_honk_record; /* DS:0x6754 */
extern volatile cb_u16 nav_choice_radio_record; /* DS:0x6756 */
extern volatile cb_u16 nav_pending_record_link; /* DS:0x675A */
/* SS:0x2B13 in the binary; runtime SS=DS makes this ordinary near data. */
extern volatile cb_u16 nav_kind2_target_offsets[];
/* DS:0x6D3E alias used as a zero-skipping, sentinel-terminated contact list. */
extern volatile cb_u16 nav_contact_slot_words[];
extern volatile cb_u16 nav_deferred_record_type; /* DS:0x6768 */
extern volatile cb_u16 nav_deferred_record_link; /* DS:0x676A */
extern volatile char nav_radio_snd_path[];    /* DS:0x0D16 */
extern volatile cb_u8 nav_presentation_reverse; /* DS:0x27E4 */
extern volatile cb_u16 nav_actor_presentation_state; /* DS:0x0A32 */
extern volatile cb_u16 nav_actor_ship_depth_offset; /* DS:0x2527 */
extern volatile cb_i16 nav_actor_zoom_counter; /* DS:0x2B93 */
extern volatile cb_i16 presentation_box_phase; /* DS:0x2B93 alias */
extern volatile cb_u8 nav_actor_completion_latch; /* DS:0x27E5 */
extern volatile cb_u16 nav_target_presentation_state; /* DS:0x0A34 */
extern volatile cb_u8 nav_actor_transition_phase; /* DS:0x2792 */
extern volatile cb_u16 nav_kind100_target_record; /* DS:0x27D5 */
extern volatile cb_u8 nav_camera_view_active; /* DS:0x278A */
extern volatile cb_u8 nav_camera_view_state; /* DS:0x278B */
extern volatile cb_u8 nav_camera_approach_phase; /* DS:0x27DF */
extern volatile cb_u8 nav_location_panel_active; /* DS:0x278C */
extern volatile cb_u16 nav_location_panel_source_width; /* DS:0x277E */
extern volatile bloodprg_rect_i16 nav_location_panel_target_rect; /* DS:0x2780 */
extern volatile cb_u8 nav_location_panel_scale_step; /* DS:0x2789 */
/* DS:0x2AAB alias shared with presentation_choice_current_rect. */
extern volatile bloodprg_rect_i16 nav_location_panel_current_rect;
extern volatile cb_u8 nav_actor_5_active; /* DS:0x278E */
extern volatile cb_u16 nav_selected_location_record; /* DS:0x27BF */
extern volatile cb_u8 nav_screen_rebuild_pending; /* DS:0x27D9 */
extern volatile cb_u8 nav_transition_pending; /* DS:0x27DA */
extern volatile cb_u8 nav_target_hover_row; /* DS/GS:0x27C7 */
extern volatile cb_u8 nav_target_selection; /* DS:0x27E7 */
extern volatile cb_u16 nav_console_selected_item; /* DS:0x2A19 */
extern volatile cb_u16 nav_bridge_seek_target_arc; /* DS:0x279B */
extern volatile cb_u8 nav_actor_0_busy; /* DS:0x2A7B */
extern volatile cb_u8 nav_actor_1_busy; /* DS:0x2A93 */
/* The binary addresses these records through BP; runtime SS=DS. */
extern volatile bloodprg_nav_actor_slot nav_actor_slots[6]; /* SS:0x2A1B */
extern bloodprg_nav_actor_handler CB_CODE_DATA
        nav_actor_handlers[6]; /* CS:0x06D4 */
extern bloodprg_nav_choice_handler CB_CODE_DATA
        nav_choice_handlers[5]; /* CS:0x0F29 */
extern cb_u32 nav_actor_live_palette_dwords[0x90]; /* DS:0x5251 */
/* The shipped dispatcher keeps ES equal to DS for this destination. */
extern cb_u32 nav_actor_bridge_palette_dwords[0x90]; /* ES:0x5B58 */
extern volatile cb_u8 presentation_mode_flag_27e0; /* DS:0x27E0 */
extern volatile cb_u8 presentation_mode_flag_27e1; /* DS:0x27E1 */
extern volatile cb_u8 presentation_mode_active; /* DS:0x27EA */
extern volatile cb_u8 presentation_completion_audio_pending; /* DS:0x27EB */
extern volatile cb_u16 presentation_mode_previous_state; /* DS:0x0A36 */
extern const bloodprg_rect_i16
        presentation_box_animation_rects[6]; /* DS:0x2B97 */
extern volatile cb_u8 presentation_choice_active; /* DS:0x259B */
extern volatile cb_u8 nav_choice_left_motion_active; /* DS:0x2736 */
extern volatile cb_u8 nav_choice_right_motion_active; /* DS:0x2737 */
extern volatile cb_u8 nav_choice_motion_active; /* DS:0x2738 */
extern volatile cb_u8 presentation_choice_phase; /* DS:0x259C */
extern const cb_u16 presentation_choice_items[]; /* DS:0x259D */
extern volatile cb_i16 nav_choice_animation_target_rect[4]; /* DS:0x253D */
extern volatile cb_i16 presentation_choice_target_rect[4]; /* DS:0x25CF */
extern volatile cb_i16 presentation_choice_current_rect[4]; /* DS:0x2AAB */
extern volatile cb_u16 list_widget_label_widths[]; /* DS:0x2AB3 */
extern const cb_u8 CB_GAME_DATA list_widget_cancel_label[]; /* GS:0x0174 */
extern const cb_u8 CB_NEAR *
        option_menu_label_pointers[]; /* DS:0x2567 */
extern const cb_u8 option_menu_music_on_label[]; /* DS:0x2578 */
extern const cb_u8 option_menu_music_off_label[]; /* DS:0x2581 */
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
#pragma aux list_widget_layout_unified \
        parm [si] value [ax] modify exact [ax]
#pragma aux nav_choice_handler_0 modify exact [ax]
#pragma aux nav_choice_handler_3 modify exact [ax si]
#pragma aux nav_choice_dispatch modify exact [ax di]
#pragma aux screen_mode_update parm [ax] modify exact [di es]
#pragma aux nav_kind2_target_list_build value [ax] modify exact [ax cx]
#endif

int CB_NEAR presentation_line_helper(
        volatile bloodprg_presentation_line_record CB_NEAR *line); /* 0x007E1C */
void CB_NEAR nav_actor_slot_update_loop(void); /* 0x007D7B */
void CB_NEAR presentation_mode_dispatch(void); /* 0x0078D0 */
void CB_NEAR camera_nav_update(void); /* 0x00792D */
void CB_NEAR screen_mode_update(
        cb_u16 queued_scene_link_target); /* 0x0079E5; original input BP */
void CB_FAR bridge_render_frame(
        cb_u16 scene_link_target); /* 0x0077E0; original input BP */
void CB_NEAR screen_flags_init(void); /* 0x00959D */
void CB_NEAR camera_fsm_state_gate(void); /* 0x008A4E */
void CB_NEAR nav_camera_state_check(void); /* 0x008CCE */
/* comparison_extent normalizes the original inherited SS:[BP+4] context. */
void CB_NEAR entity_draw_full(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent);
        /* 0x009240 */
void CB_NEAR mode_gate_27e8(void); /* 0x008BAB */
void CB_NEAR nav_state_gate(void); /* 0x0082E8 */
void CB_NEAR nav_choice_dispatch(void); /* 0x0085E2 */
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
void CB_NEAR nav_choice_handler_1(void); /* 0x00872C */
void CB_NEAR nav_choice_handler_2(void); /* 0x0087BD */
void CB_NEAR nav_choice_handler_3(void); /* 0x008848 */
void CB_NEAR nav_choice_handler_4(void); /* 0x00886C */
cb_u16 CB_FAR nav_kind2_target_list_build(void); /* 0x0071CF */

#endif
