#ifndef BLOODPRG_SHIP3D_H
#define BLOODPRG_SHIP3D_H

#include "bloodprg_common.h"
#include "bloodprg_vm.h"

#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8 0x0008u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10 0x0010u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40 0x0040u
#define SHIP_3D_OBJECT_KIND_POSITION_KIND100 0x0100u
#define SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200 0x0200u

#define SHIP_3D_FIELD_SELECTOR_POSITION 0x000bu
#define SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH 0x0009u
#define SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH 0x000au
#define SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD 0x000cu
#define SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD 0x000eu
#define SHIP_3D_FIELD_SELECTOR_PARENT_LINK 0x0011u
#define SHIP_3D_SOURCE_BITSET_SELECTOR 0x0005u
#define SHIP_3D_SOURCE_BITSET_KIND 0x0002u
#define SHIP_3D_PRESENTABLE_KIND_MASK 0x0098u
#define SHIP_3D_OBJECT_NAME_OFFSET 4u
#define SHIP_3D_HUD_PALETTE_FIRST 128u
#define SHIP_3D_HUD_PALETTE_COLORS 64u
#define SHIP_3D_HUD_PALETTE_BYTES (SHIP_3D_HUD_PALETTE_COLORS * 3u)
#define SHIP_3D_HUD_LAYOUT_NAME_BYTES 16u
#define SHIP_PRESENTATION_ACTIVE 0x0001u
#define SHIP_PRESENTATION_DIALOGUE 0x0002u
#define SHIP_PRESENTATION_HUD 0x0004u
#define SHIP_PRESENTATION_TRAVEL 0x0008u
#define SHIP_PRESENTATION_NAVIGATION 0x0010u
#define SHIP_PRESENTATION_PHASE_MASK 0x001eu
#define SHIP_PRESENTATION_DIALOGUE_LINE_END 6u
#define SHIP_PRESENTATION_TRANSITION_COMPLETE 100u
#define SHIP_3D_HUD_OFFSCREEN_COORD ((cb_i16)-1000)
#define SHIP_3D_POINT_CLOUD_COUNT 1000u
#define SHIP_3D_OBJECT_ANCHOR_COUNT 11u
#define SHIP_3D_NAV_ENTITY_BASE 0x0015u
#define BRIDGE_PANORAMA_STATION_COUNT 4u

typedef struct bridge_panorama_directory_entry {
    cb_u32 file_offset;
    cb_u32 byte_count;
} bridge_panorama_directory_entry;

typedef struct bridge_panorama_station_record {
    cb_u8 prefix[12];
    cb_i16 orb_box[4];
    cb_u8 suffix[4];
} bridge_panorama_station_record;

typedef char bridge_panorama_directory_entry_size_must_be_8[
        sizeof(bridge_panorama_directory_entry) == 8 ? 1 : -1];
typedef char bridge_panorama_station_record_size_must_be_24[
        sizeof(bridge_panorama_station_record) == 24 ? 1 : -1];

typedef struct ship_3d_projection_context {
    cb_i32 matrix[9];
    cb_u16 projected_x;
    cb_u16 projected_y;
    cb_u16 projected_depth;
    cb_u16 depth_scale;
} ship_3d_projection_context;

typedef struct ship_3d_projection_terms {
    cb_i32 b_cos;
    cb_i32 b_sin;
    cb_i32 c_cos;
    cb_i32 c_sin;
    cb_i32 a_cos;
    cb_i32 a_sin;
} ship_3d_projection_terms;

typedef struct ship_3d_angle_table_entry {
    cb_i16 cosine;
    cb_i16 sine;
} ship_3d_angle_table_entry;

typedef struct ship_3d_point_record {
    cb_u16 x;
    cb_u16 y;
    cb_u16 z;
    cb_u16 scratch;
} ship_3d_point_record;

typedef struct ship_3d_object_anchor {
    cb_u16 x;
    cb_u16 y;
    cb_u16 z;
} ship_3d_object_anchor;

typedef char ship_3d_projection_context_size_must_be_44[
        sizeof(ship_3d_projection_context) == 44 ? 1 : -1];
typedef char ship_3d_point_record_size_must_be_8[
        sizeof(ship_3d_point_record) == 8 ? 1 : -1];
typedef char ship_3d_object_anchor_size_must_be_6[
        sizeof(ship_3d_object_anchor) == 6 ? 1 : -1];

typedef struct ship_3d_matrix_slot {
    cb_u16 first_word;
    cb_u8 tail[22];
} ship_3d_matrix_slot;

typedef struct ship_3d_position_field {
    cb_u16 x;
    cb_u16 y;
} ship_3d_position_field;

typedef union ship_3d_hud_layout_name {
    char text[SHIP_3D_HUD_LAYOUT_NAME_BYTES];
    cb_u16 first_word;
} ship_3d_hud_layout_name;

typedef struct ship_3d_hud_layout_entry {
    ship_3d_hud_layout_name name;
    cb_u16 resource_id;
    cb_u16 entity_id;
    cb_u8 active;
    cb_u8 reserved;
} ship_3d_hud_layout_entry;

typedef char ship_3d_hud_layout_entry_size_must_be_22[
        sizeof(ship_3d_hud_layout_entry) == 22 ? 1 : -1];

extern volatile cb_u16 CB_GAME_DATA ship_3d_depth_offset; /* DS/GS:0x2527 */
extern volatile cb_u8 CB_GAME_DATA
        ship_3d_plane_blit_crop_enabled; /* DS/GS:0x252E */
extern volatile cb_u8
        ship_3d_plane_blit_crop_enabled_ds; /* DS:0x252E alias */
extern volatile cb_u8 ship_3d_depth_opening; /* DS:0x252F */
extern volatile cb_u8 ship_3d_depth_closing; /* DS:0x2530 */
extern volatile cb_u8 ship_3d_depth_step;    /* DS:0x2531 */
extern volatile cb_u8 ship_3d_scene_dispatch_blocked; /* DS:0x252D */
extern volatile cb_u8 ship_3d_hud_initialized; /* DS:0x2529 */
extern volatile cb_u8 ship_3d_hud_init_pending; /* DS:0x2535 */
extern volatile cb_u8 ship_3d_exit_pending; /* DS:0x2532 */
extern volatile cb_u8 ship_3d_navigation_trigger; /* DS:0x27D8 alias */
extern volatile cb_u8 ship_3d_navigation_snapshot_pending; /* DS:0x2739 */
extern volatile cb_u8 ship_3d_dialogue_phase_ready; /* DS:0x2534 */
extern volatile cb_u16 ship_3d_dialogue_cycle_line; /* DS:0x24F5 */
extern volatile cb_u8 ship_3d_alien_overlay_armed; /* DS:0x0AE3 */
extern volatile cb_u8 ship_3d_temp_snd_trigger; /* DS:0x0AE4 */
extern volatile cb_u8 ship_3d_nav_choice_sound_gate; /* DS:0x0B13 */
extern volatile cb_u16 ship_3d_nav_choice_target_y; /* DS:0x253F */
extern volatile cb_u16 ship_3d_target_layout_center_x; /* DS:0x0AC6 */
extern volatile cb_u8 ship_3d_target_layout_preserve_widths; /* DS:0x0ADC */
extern volatile cb_u8 ship_3d_target_layout_extra_entry; /* DS:0x0ADD */
extern volatile cb_u8 ship_3d_interpolation_duration; /* DS:0x0ADA */
extern volatile cb_u16 ship_3d_current_target; /* DS:0x251B */
extern volatile cb_u8 ship_3d_target_select_phase; /* DS:0x252B */
extern volatile cb_u8 ship_3d_target_fallback; /* DS:0x252C */
extern const cb_u16 ship_3d_fallback_target_table[]; /* DS:0x2537 */
extern const bloodprg_rect_i16
        ship_3d_target_transition_rect; /* DS:0x2545 */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_b; /* GS:0x2F6D */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_c; /* GS:0x2F6F */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_angle_a; /* GS:0x2F71 */
/* Original BP addressing selects SS:0x2A1B; medium-model C requires SS == DS. */
extern volatile ship_3d_matrix_slot ship_3d_matrix_slots[];
extern volatile ship_3d_projection_terms CB_GAME_DATA ship_3d_projection_inputs; /* GS:0x2F7D */
extern volatile ship_3d_projection_context CB_GAME_DATA ship_3d_projection; /* GS:0x2F95 */
extern volatile cb_u16 CB_GAME_DATA ship_3d_projection_remaining; /* GS:0x2F77 */
extern volatile ship_3d_point_record CB_GAME_DATA ship_3d_point_cloud[]; /* GS:0x2FC1 */
extern volatile ship_3d_point_record CB_GAME_DATA
        ship_3d_projection_work; /* GS:0x4F01 */
extern volatile ship_3d_object_anchor CB_GAME_DATA
        ship_3d_object_anchors[]; /* GS:0x4F09 */
/* Original BP indexing selects SS:0x4F45; GAME_DATA must bind to SS == GS. */
extern const ship_3d_angle_table_entry CB_GAME_DATA ship_3d_angle_table[];
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_left;    /* GS:0x5235 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_right;   /* GS:0x5237 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_top;     /* GS:0x5239 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_clip_bottom;  /* GS:0x523B */
extern volatile cb_u8 CB_GAME_DATA
        ship_3d_hud_palette_snapshot[SHIP_3D_HUD_PALETTE_BYTES]; /* GS:0x5CD8 */
extern const cb_u32 ship_3d_hud_pyramid_palette_dwords[0x30]; /* DS:0x5D98 */
extern cb_u32 CB_GAME_DATA ship_3d_hud_palette_stage_dwords[0x30]; /* GS:0x5491 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_x; /* GS:0x2F65 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_y; /* GS:0x2F67 */
extern volatile cb_i16 CB_GAME_DATA ship_3d_camera_z; /* GS:0x2F69 */
extern volatile cb_u16 bridge_panorama_file_handle; /* DS:0x0AC4 */
extern volatile bridge_panorama_directory_entry
        bridge_panorama_directory; /* DS:0x0AD2 */
extern volatile bridge_panorama_station_record CB_GAME_DATA
        bridge_panorama_stations[]; /* GS:0x2A1B */
/* Helper output is SS:0x6886; the filter reads it through DS after DS=GS. */
extern volatile cb_u16 CB_GAME_DATA ship_3d_nav_source_offsets[];
/* Original BP writes SS:0x250B; the shipped data group has SS == DS == GS. */
extern volatile cb_u16 ship_3d_presentable_name_offsets[];
/* Original output uses SS:0x24FB; the shipped runtime has SS == GS. */
extern volatile cb_u16 CB_GAME_DATA vm_arche_position_match_offsets[];
/* Filter output is SS/DS:0x2B53 under the shipped SS=DS=GS data-group alias. */
extern volatile cb_u16 ship_3d_navigation_candidate_offsets[];
extern const cb_u16 ship_3d_navigation_trigger_target_list[]; /* DS:0x253B */
/* Original clear uses SS:0x2BC7; later lookup uses GS:0x2BC7 (SS == GS). */
extern volatile ship_3d_hud_layout_entry CB_GAME_DATA ship_3d_hud_layout[];

#if defined(__WATCOMC__)
#pragma aux binary_u32_sqrt parm [dx ax] value [ax] modify exact [ax]
#pragma aux ship_3d_position_distance parm [si] [di] [dx] value [ax] modify exact [ax]
#pragma aux ship_3d_position_field_resolve parm [si] [dx] value [ax] modify exact [ax]
#pragma aux ship_3d_object_table_bit_test_full parm [ax] [si] value [ax] modify exact [ax]
/* Open Watcom C16 reserves BP, so the natural-C boundary uses BX for the
 * output cursor; an integration adapter must translate the binary's BP ABI. */
#pragma aux ship_3d_nav_source_list_build_full parm [es di] [bx] value [bx] modify exact [bx]
#pragma aux ship_3d_navigation_candidate_build parm [es di] modify exact [bx di es]
#pragma aux vm_state_record_processor modify exact [ax]
#pragma aux ship_3d_presentable_name_list_build parm [es di] value [bp] modify exact [bp]
#pragma aux matrix_table_clear_2a1b modify exact []
#pragma aux ship_3d_projection_matrix_build modify exact [ax es]
#pragma aux ship_3d_point_cloud_randomize modify exact [ax cx es]
#pragma aux ship_3d_depth_scroll_step modify exact [ax]
#pragma aux ship_3d_hud_palette_snapshot_and_camera_reset \
        modify exact [bx dx]
#pragma aux ship_3d_point_cloud_project modify exact []
#pragma aux ship_3d_object_sprite_project modify exact []
#pragma aux ship_3d_plane_band_copy modify exact []
#pragma aux ship_3d_target_record_select value [ax] modify exact [ax]
#pragma aux draw_hud_element_2bc7 modify exact []
#pragma aux bridge_panorama_frame_load parm [ax] modify exact []
#pragma aux page_flip value [ax] modify exact [ax bx]
#pragma aux alien_overlay_cycle modify exact [ax dx si di bp]
#endif

cb_u16 CB_FAR binary_u32_sqrt(cb_u32 value); /* 0x002E33 */
cb_u16 CB_NEAR ship_3d_position_distance(
        const volatile bloodprg_vm_object_header CB_NEAR *first_record,
        const volatile bloodprg_vm_object_header CB_NEAR *second_record,
        cb_u16 inherited_kind100_compare_word); /* 0x0060DD */
volatile ship_3d_position_field CB_NEAR *CB_NEAR
ship_3d_position_field_resolve(
        volatile bloodprg_vm_object_header CB_NEAR *record,
        cb_u16 kind100_compare_word);        /* 0x0061A6 */
int CB_NEAR ship_3d_object_table_bit_test_full(cb_u16 object_offset,
        const volatile cb_u8 CB_NEAR *bitset_base); /* 0x006210 */
cb_u16 CB_NEAR *CB_FAR ship_3d_nav_source_list_build_full(
        const volatile bloodprg_vm_object_header CB_FAR *target,
        cb_u16 CB_NEAR *output);              /* 0x00624B */
void CB_FAR ship_3d_navigation_candidate_build(
        const volatile bloodprg_vm_object_header CB_FAR *target);
                                                /* 0x0070EE */
void CB_SAVE_REGS CB_FAR vm_state_record_processor(void); /* 0x00713D */
volatile cb_u16 CB_NEAR *CB_FAR ship_3d_presentable_name_list_build(
        const volatile bloodprg_vm_object_header CB_FAR *target);
                                                /* 0x007259 */
void CB_FAR matrix_table_clear_2a1b(void);     /* 0x00963F */
void CB_FAR ship_3d_projection_matrix_build(void); /* 0x0098B9 */
void CB_FAR ship_3d_point_cloud_project(void); /* 0x009A10 */
void CB_FAR ship_3d_object_sprite_project(void); /* 0x009B98 */
void CB_FAR ship_3d_point_cloud_randomize(void); /* 0x009B67 */
void CB_NEAR bridge_panorama_frame_load(cb_u16 frame); /* 0x00981B */
cb_u16 CB_FAR page_flip(void); /* 0x00954A */
void CB_NEAR ship_3d_depth_scroll_step(void); /* 0x00B75C */
cb_u16 CB_NEAR ship_3d_target_record_select(void); /* 0x00B2BB */
void CB_FAR draw_hud_element_2bc7(void); /* 0x006FF3 */
void CB_FAR ship_3d_hud_palette_snapshot_and_camera_reset(void); /* 0x008C96 */
extern volatile cb_u16 bridge_mouse_arc; /* DS:0x2797 */
extern volatile cb_u16 bridge_seek_initial_distance; /* DS:0x279D */
extern volatile cb_u8 bridge_turn_direction; /* DS:0x27DB */
extern volatile cb_u16 bridge_frame_angle_bias; /* DS:0x27A7 */
int CB_FAR bridge_steer_update(
        cb_u16 CB_NEAR *presentation_link_target); /* 0x009656 */
void CB_FAR alien_overlay_cycle(void); /* 0x00B591 */
void CB_FAR ship_3d_plane_band_copy(void); /* 0x00B6DD */
void CB_FAR ship_presentation_fsm(void); /* 0x00AFA0 */
void CB_NEAR ship_3d_hud_init(void); /* 0x00B079 */
void CB_NEAR ship_3d_navigation_update(void); /* 0x00B34E */
/* Original context is SS:BP and the framebuffer is normalized ES:0. */
void CB_NEAR ship_3d_plot_point(
        const volatile ship_3d_projection_context CB_GAME_DATA *projection,
        volatile cb_u8 CB_FAR *framebuffer);   /* 0x009B04 */

#endif
