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

typedef struct bloodprg_nav_chart_point {
    cb_u16 x;
    cb_u16 y;
} bloodprg_nav_chart_point;

typedef struct bloodprg_nav_chart_object {
    cb_u16 kind;
    cb_u8 reserved_02[0x12];
    cb_u16 endpoint_context;
    cb_u8 reserved_16[2];
    bloodprg_nav_chart_point marker[2];
} bloodprg_nav_chart_object;

typedef struct bloodprg_nav_chart_arche {
    cb_u8 reserved_00[0x22];
    cb_u16 endpoint_context;
} bloodprg_nav_chart_arche;

typedef struct bloodprg_location_panel_art_entry {
    char name[16];
    cb_u16 resource_id;
    cb_u16 group;
    cb_u16 reserved_14;
} bloodprg_location_panel_art_entry;

typedef struct bloodprg_nav_wipe_point {
    cb_i16 x;
    cb_i16 y;
} bloodprg_nav_wipe_point;

typedef union bloodprg_name_area_effect_control {
    cb_u16 word;
    struct {
        cb_u8 operation;
        cb_u8 frames_remaining;
    } fields;
} bloodprg_name_area_effect_control;

typedef struct bloodprg_name_area_effect_frame {
    cb_u16 x;
    cb_u16 y;
    cb_u16 width;
    cb_u16 height;
} bloodprg_name_area_effect_frame;

typedef struct bloodprg_name_area_effect_sequence {
    bloodprg_name_area_effect_control control;
    bloodprg_name_area_effect_frame frames[1];
} bloodprg_name_area_effect_sequence;

extern volatile cb_u8 name_area_effect_active_ds; /* DS:0x27E8 */
extern volatile cb_u8 name_area_effect_restart; /* DS:0x27E9 */
extern volatile cb_u8 CB_GAME_DATA
        name_area_effect_active_gs; /* explicit GS:0x27E8 alias */
extern volatile cb_u8 CB_GAME_DATA
        name_area_effect_restart_gs; /* explicit GS:0x27E9 alias */
extern const bloodprg_name_area_effect_frame CB_NEAR
        *name_area_effect_frame_cursor; /* DS:0x27ED */
extern volatile bloodprg_name_area_effect_control
        name_area_effect_control; /* DS:0x27EF */
extern volatile cb_u8 CB_GAME_DATA
        name_area_effect_operation_gs; /* explicit GS:0x27EF alias */
extern const bloodprg_name_area_effect_sequence CB_NEAR
        *name_area_effect_sequences[10]; /* DS:0x27F1 */

typedef struct bloodprg_nav_wipe_span {
    cb_u16 left;
    cb_u16 width;
} bloodprg_nav_wipe_span;

typedef void (CB_NEAR *bloodprg_nav_actor_handler)(
        volatile bloodprg_presentation_line_record CB_NEAR *line);
typedef void (CB_NEAR *bloodprg_nav_choice_handler)(void);

typedef char bloodprg_nav_actor_slot_size_must_be_24[
        sizeof(bloodprg_nav_actor_slot) == 24 ? 1 : -1];
typedef char bloodprg_nav_chart_point_size_must_be_4[
        sizeof(bloodprg_nav_chart_point) == 4 ? 1 : -1];
typedef char bloodprg_nav_chart_object_size_must_be_32[
        sizeof(bloodprg_nav_chart_object) == 32 ? 1 : -1];
typedef char bloodprg_nav_chart_arche_size_must_be_36[
        sizeof(bloodprg_nav_chart_arche) == 36 ? 1 : -1];
typedef char bloodprg_location_panel_art_size_ok[
        sizeof(bloodprg_location_panel_art_entry) == 22 ? 1 : -1];
typedef char bloodprg_nav_wipe_point_size_must_be_4[
        sizeof(bloodprg_nav_wipe_point) == 4 ? 1 : -1];
typedef char bloodprg_name_area_control_size_ok[
        sizeof(bloodprg_name_area_effect_control) == 2 ? 1 : -1];
typedef char bloodprg_name_area_effect_frame_size_must_be_8[
        sizeof(bloodprg_name_area_effect_frame) == 8 ? 1 : -1];
typedef char bloodprg_nav_wipe_span_size_must_be_4[
        sizeof(bloodprg_nav_wipe_span) == 4 ? 1 : -1];

extern volatile cb_u8 nav_choice_phase;       /* DS:0x2565 */
extern volatile cb_u16 nav_choice_honk_record; /* DS:0x6754 */
extern volatile cb_u16 nav_choice_radio_record; /* DS:0x6756 */
extern volatile cb_u16 nav_pending_record_link; /* DS:0x675A */
extern volatile cb_u16 CB_GAME_DATA
        nav_pending_record_link_gs; /* explicit GS:0x675A alias */
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
extern volatile cb_u8 CB_GAME_DATA
        nav_actor_transition_phase_gs; /* explicit GS:0x2792 alias */
extern volatile cb_u16 nav_kind100_target_record; /* DS:0x27D5 */
extern volatile cb_u8 nav_camera_view_active; /* DS:0x278A */
extern volatile cb_u8 nav_camera_view_state; /* DS:0x278B */
extern volatile cb_u8 nav_camera_approach_phase; /* DS:0x27DF */
extern volatile cb_u8 CB_GAME_DATA
        nav_camera_view_active_gs; /* explicit GS:0x278A alias */
extern volatile cb_u8 CB_GAME_DATA
        nav_camera_view_state_gs; /* explicit GS:0x278B alias */
extern volatile cb_u8 CB_GAME_DATA
        nav_camera_approach_phase_gs; /* explicit GS:0x27DF alias */
extern volatile cb_u8 nav_location_panel_active; /* DS:0x278C */
extern volatile cb_u16 nav_location_panel_source_width; /* DS:0x277E */
extern volatile cb_u16 nav_chart_pick_width; /* DS:0x277A */
extern volatile cb_u16 nav_chart_pick_height; /* DS:0x277C */
extern volatile bloodprg_rect_i16 nav_location_panel_target_rect; /* DS:0x2780 */
extern volatile cb_u8 nav_location_panel_transition_state; /* DS:0x2788 */
extern volatile cb_u8 nav_location_panel_scale_step; /* DS:0x2789 */
/* DS:0x2AAB alias shared with presentation_choice_current_rect. */
extern volatile bloodprg_rect_i16 nav_location_panel_current_rect;
extern const bloodprg_location_panel_art_entry
        nav_location_panel_art_table[]; /* DS:0x2BC7 */
extern const cb_u8 nav_location_panel_planet_label[]; /* DS:0x012E */
extern const cb_u8 nav_location_panel_ship_label[]; /* DS:0x0137 */
extern const cb_u8 nav_location_panel_black_hole_label[]; /* DS:0x013E */
extern const cb_u8 nav_life_support_label[]; /* DS:0x014B */
extern const cb_u8 CB_GAME_DATA
        nav_life_support_label_gs[]; /* GS:0x014B alias */
extern volatile cb_u8 nav_actor_5_active; /* DS:0x278E */
extern volatile cb_u16 nav_selected_location_record; /* DS:0x27BF */
extern volatile cb_u16 nav_chart_object_count; /* DS:0x27C1 */
extern volatile cb_u8 nav_chart_secondary_marker; /* DS:0x278F */
extern volatile cb_u8 nav_chart_subobject_count; /* DS:0x2790 */
extern volatile cb_u8 nav_center_wipe_complete; /* DS:0x2791 */
extern volatile cb_u8 nav_chart_entity_state_mask; /* DS:0x0B3F */
extern volatile cb_u8 CB_GAME_DATA
        nav_chart_entity_state_mask_gs; /* explicit GS:0x0B3F alias */
extern const bloodprg_nav_wipe_point
        nav_center_wipe_endpoints[9]; /* DS:0x2752 */
extern volatile cb_u8 nav_screen_rebuild_pending; /* DS:0x27D9 */
extern volatile cb_u8 CB_GAME_DATA
        nav_screen_rebuild_pending_gs; /* explicit GS:0x27D9 alias */
extern volatile cb_u8 nav_transition_pending; /* DS:0x27DA */
extern volatile cb_u8 nav_target_hover_row; /* DS/GS:0x27C7 */
extern volatile cb_u8 nav_target_selection; /* DS:0x27E7 */
extern volatile cb_u16 nav_console_selected_item; /* DS:0x2A19 */
extern volatile cb_u16 nav_bridge_seek_target_arc; /* DS:0x279B */
extern volatile cb_u8 nav_actor_0_busy; /* DS:0x2A7B */
extern volatile cb_u8 CB_GAME_DATA
        nav_actor_0_busy_gs; /* explicit GS:0x2A7B alias */
extern volatile cb_u8 nav_actor_1_busy; /* DS:0x2A93 */
/* The binary addresses these records through BP; runtime SS=DS. */
extern volatile bloodprg_nav_actor_slot nav_actor_slots[6]; /* SS:0x2A1B */
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
extern volatile bloodprg_rect_i16
        presentation_word_choice_target_rect; /* DS:0x254D */
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
extern volatile cb_u8 CB_FAR *nav_resource_buffer; /* DS:0x0A80 */
extern volatile cb_u8 CB_FAR * CB_GAME_DATA
        nav_resource_buffer_gs; /* explicit GS:0x0A80 alias */
extern volatile char CB_FS_DATA
        fs_presentation_resource_names[][16]; /* FS:0x0C04 */

#if defined(__WATCOMC__)
#pragma aux nav_choice_handler_0 modify exact [ax]
#pragma aux nav_choice_dispatch modify exact [ax di]
#pragma aux screen_mode_update parm [ax] modify exact [ax di es]
#pragma aux nav_kind2_target_list_build value [ax] modify exact [ax cx]
#pragma aux nav_chart_object_pick \
        parm [es di] value [ax] modify exact [ax bx cx dx bp di]
#pragma aux name_area_palette_effect_update modify exact [ax]
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
void CB_NEAR camera_fsm_state_gate(
        cb_u16 scene_link_target); /* 0x008A4E; original input BP */
void CB_NEAR nav_camera_state_check(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent);
        /* 0x008CCE; original context is inherited through SS:BP. */
/* comparison_extent normalizes the original inherited SS:[BP+4] context. */
void CB_NEAR entity_draw_full(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent);
        /* 0x009240 */
/* comparison_extent is the inherited context forwarded to entity_draw_full. */
void CB_NEAR location_info_panel_dispatch(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent);
        /* 0x009083 */
void CB_NEAR name_area_palette_effect_update(void); /* 0x008BAB */
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
        const cb_u16 CB_NEAR *items,
        const volatile void CB_FAR *label_segment_anchor); /* 0x008428 */
void CB_NEAR presentation_choice_transition_step(void); /* 0x001AD3 */
void CB_NEAR confirm_dialog_step(void); /* 0x0014CA */
void CB_NEAR nav_choice_handler_0(void); /* 0x008713 */
void CB_NEAR nav_choice_handler_1(void); /* 0x00872C */
void CB_NEAR nav_choice_handler_2(void); /* 0x0087BD */
void CB_NEAR nav_choice_handler_3(void); /* 0x008848 */
void CB_NEAR nav_choice_handler_4(void); /* 0x00886C */
cb_u16 CB_FAR nav_kind2_target_list_build(void); /* 0x0071CF */
/* record_base normalizes the original implicit ES record-table segment. */
cb_u16 CB_NEAR nav_chart_object_pick(
        const volatile cb_u8 CB_FAR *record_base); /* 0x0092A3 */
void CB_NEAR nav_center_wipe_span_table_build(
        const volatile bloodprg_nav_wipe_point CB_NEAR *endpoint); /* 0x009364 */

#endif
