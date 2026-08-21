#ifndef BLOODPRG_VM_H
#define BLOODPRG_VM_H

#include "bloodprg_common.h"
#include "bloodprg_platform.h"
#include "bloodprg_random.h"

typedef union bloodprg_vm_ui_state {
    cb_u16 word;
    struct {
        cb_u8 flags;
        cb_u8 auxiliary;
    } bytes;
} bloodprg_vm_ui_state;

typedef volatile cb_u8 CB_FAR *bloodprg_vm_image_ptr;

typedef bloodprg_vm_image_ptr CB_NEAR bloodprg_vm_opcode_handler(
        bloodprg_vm_image_ptr script_bytes);

#define BLOODPRG_VM_RESOURCE_COUNT 5u
#define BLOODPRG_VM_RESOURCE_PROFILE_COUNT 6u
#define BLOODPRG_VM_SPECIAL_SLOT_COUNT 16u
#define BLOODPRG_VM_OPCODE_MIN 0xa0u
#define BLOODPRG_VM_OPCODE_MAX 0xd2u
#define BLOODPRG_VM_OPCODE_WINDOW_COUNT 0x60u
#define BLOODPRG_VM_CONCEPT_OPCODE 0xa3u
#define BLOODPRG_VM_TEXT_OPCODE 0xa6u
#define BLOODPRG_VM_OPTION_PREFIX 0xa1u
#define BLOODPRG_VM_STREAM_END 0xffu
#define BLOODPRG_VM_SKIP_COUNT_MASK 0x0fu
#define BLOODPRG_VM_RESUME_ACTIVE 0x02u
#define BLOODPRG_VM_HISTORY_RING_MASK 0x0fu
#define BLOODPRG_VM_YIELD_STOP_BLOCK 0x01u
#define BLOODPRG_VM_YIELD_CONTINUE 0x02u
#define BLOODPRG_VM_YIELD_SAVE_CURSOR 0x03u

typedef cb_u16 bloodprg_vm_resource_profile[BLOODPRG_VM_RESOURCE_COUNT];

#define vm_ui_flags (vm_ui_state.bytes.flags)

extern volatile cb_u8 vm_sequence_active;    /* GS:0x252A */
extern volatile cb_u8 vm_sequence_active_ds; /* DS:0x252A alias */
extern volatile cb_u8 CB_GAME_DATA
        vm_sequence_active_gs;               /* explicit GS:0x252A alias */
extern volatile cb_u8 vm_ship_3d_depth_step; /* GS:0x2531 */
extern volatile cb_u8 CB_GAME_DATA
        vm_ship_3d_depth_step_gs;            /* explicit GS:0x2531 alias */
extern volatile cb_u16 vm_ship_active_flags; /* GS:0x24F3 */
extern volatile cb_u16 CB_GAME_DATA
        vm_ship_active_flags_gs; /* explicit GS:0x24F3 alias */
extern volatile cb_u8 vm_ship_active_flags_low; /* game data:0x24F3 */
extern volatile cb_u16 vm_dialogue_hold_countdown; /* game data:0x0B35 */
extern volatile cb_u8 vm_scene_gate;         /* GS:0x274F */
extern volatile cb_u8 CB_GAME_DATA
        vm_scene_gate_gs;                    /* explicit GS:0x274F alias */
extern volatile cb_u16 vm_scene_record_offset; /* DS:0x274D */
extern volatile bloodprg_vm_ui_state vm_ui_state; /* game data:0x2793 */
extern volatile cb_i16 vm_bridge_view_frame; /* game data:0x2795 */
extern volatile cb_u8 vm_bridge_redraw_pending; /* GS:0x27D8 */
extern volatile cb_u8 CB_GAME_DATA
        vm_bridge_redraw_pending_gs; /* explicit GS:0x27D8 alias */
extern volatile cb_u8 CB_GAME_DATA vm_subtitle_display_mode; /* GS:0x27E2 */
extern volatile cb_u8 vm_subtitle_display_mode_ds; /* DS:0x27E2 alias */
extern volatile cb_u16 vm_operand_word_count; /* GS:0x27CF */
extern volatile cb_u16 CB_GAME_DATA
        vm_operand_word_count_gs;            /* explicit GS:0x27CF alias */
extern volatile cb_u16 vm_text_menu_inline_x; /* DS/GS:0x27D1 */
extern volatile cb_u8 vm_load_string_buffer[]; /* SS:0x2120 here; SS=GS at runtime */
extern volatile cb_u8 CB_GAME_DATA
        vm_load_string_buffer_gs[];          /* explicit GS/SS:0x2120 alias */
extern volatile cb_u8 vm_dialog_gate_0b3b;   /* GS:0x0B3B */
extern volatile cb_u8 CB_GAME_DATA
        vm_dialog_gate_0b3b_gs;              /* explicit GS:0x0B3B alias */
extern volatile cb_u8 vm_text_mode_0cf9;     /* GS:0x0CF9 */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_mode_0cf9_gs;                /* explicit GS:0x0CF9 alias */
extern volatile cb_u8 vm_text_mode_0cfa;     /* GS:0x0CFA */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_mode_0cfa_gs;                /* explicit GS:0x0CFA alias */
extern volatile cb_u8 vm_text_voice_trigger; /* GS:0x0CFB */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_voice_trigger_gs;            /* explicit GS:0x0CFB alias */
extern volatile char vm_text_buffer[];       /* GS:0x0E18 */
extern volatile char CB_GAME_DATA
        vm_text_buffer_gs[];                 /* explicit GS:0x0E18 alias */
extern volatile cb_i16 vm_text_selector;     /* GS:0x1FAB */
extern volatile cb_i16 CB_GAME_DATA
        vm_text_selector_gs;                 /* explicit GS:0x1FAB alias */
extern volatile cb_u8 vm_text_menu_pending;  /* GS:0x1FB3 */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_menu_pending_gs;             /* explicit GS:0x1FB3 alias */
extern volatile cb_u8 vm_c2_presentation_gate; /* GS:0x1FB2 */
extern volatile cb_u8 CB_GAME_DATA
        vm_c2_presentation_gate_gs;          /* explicit GS:0x1FB2 alias */
extern volatile char CB_NEAR * volatile
        vm_loaded_scene_image_path; /* DS:0x1FA3 */
extern volatile cb_u16 CB_GAME_DATA
        vm_loaded_scene_image_path_offset_gs; /* explicit GS:0x1FA3 alias */
extern volatile cb_u16 vm_text_menu_end;     /* GS:0x27D3 */
extern volatile cb_u16 CB_GAME_DATA
        vm_text_menu_end_gs;                 /* explicit GS:0x27D3 alias */
extern volatile cb_u16 vm_text_reveal_cursor; /* GS:0x5E58 */
extern volatile cb_u16 CB_GAME_DATA
        vm_text_reveal_cursor_gs;             /* explicit GS:0x5E58 alias */
extern volatile cb_u16 CB_GAME_DATA vm_text_reveal_phase; /* GS:0x5E65 */
extern volatile cb_u8 vm_text_display_active; /* GS:0x5E64 */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_display_active_gs; /* explicit GS:0x5E64 alias */
extern const char CB_FAR *vm_dic_words; /* GS:0x6728 */
extern const char CB_FAR * CB_GAME_DATA
        vm_dic_words_gs;                /* explicit GS:0x6728 alias */
extern volatile cb_u8 CB_FAR *vm_record_base; /* GS:0x6724 */
extern volatile cb_u8 CB_FAR * CB_GAME_DATA
        vm_record_base_gs; /* explicit GS:0x6724 alias */
/* BP addresses this table through SS; the shipped runtime has SS=GS. */
extern volatile cb_u16 CB_GAME_DATA
        vm_resource_handles[BLOODPRG_VM_RESOURCE_COUNT]; /* GS:0x6712 */
extern const bloodprg_vm_resource_profile CB_FS_DATA
        vm_resource_profiles[BLOODPRG_VM_RESOURCE_PROFILE_COUNT];
        /* FS:0x11F4; span:0x003C */
/* These five pointers alias the individually named 0x671c..0x672f globals. */
extern bloodprg_vm_image_ptr CB_GAME_DATA
        vm_resource_images[BLOODPRG_VM_RESOURCE_COUNT]; /* GS:0x671C */
extern bloodprg_vm_image_ptr CB_GAME_DATA vm_script_image; /* GS:0x671C */
extern bloodprg_vm_image_ptr CB_GAME_DATA vm_code_image; /* GS:0x6720 */
extern volatile cb_u16 vm_branch_stack[];    /* SS:0x6820; SS=GS at runtime */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764; SS alias in 0x6596 */
extern volatile cb_u16 CB_GAME_DATA
        vm_branch_stack_gs[];                /* explicit GS/SS:0x6820 alias */
extern volatile cb_u16 CB_GAME_DATA
        vm_resume_value_gs;                  /* explicit GS:0x6764 alias */
extern const cb_u16 CB_FAR * volatile vm_text_menu_words; /* GS:0x674A */
extern const cb_u16 CB_FAR * volatile CB_GAME_DATA
        vm_text_menu_words_gs;               /* explicit GS:0x674A alias */
extern const cb_u16 vm_presentation_menu_words_buffer[]; /* DS:0x6790 */
extern volatile cb_u16 vm_arche_record_offset; /* GS:0x6752 */
extern volatile cb_u16 CB_GAME_DATA vm_named_orxx_object_gs; /* GS:0x6750 */
extern volatile cb_u16 vm_named_orxx_object; /* DS:0x6750 alias */
extern volatile cb_u16 CB_GAME_DATA vm_arche_record_offset_gs; /* GS:0x6752 */
extern volatile cb_u16 CB_GAME_DATA vm_named_honk_object_gs; /* GS:0x6754 */
extern volatile cb_u16 vm_named_honk_object; /* DS:0x6754 alias */
extern volatile cb_u16 vm_named_menu_object; /* DS:0x6756 */
extern volatile cb_u16 vm_named_ark_object; /* DS:0x6758 */
extern volatile cb_u16 CB_GAME_DATA
        vm_named_ark_object_gs;             /* explicit GS:0x6758 alias */
extern volatile cb_u16 vm_wildcard_ref_value; /* GS:0x674E */
extern volatile cb_u16 CB_GAME_DATA
        vm_wildcard_ref_value_gs; /* explicit GS:0x674E alias */
extern volatile cb_u16 vm_block_match_value; /* GS:0x6762; SS alias in 0x6596 */
extern volatile cb_u16 CB_GAME_DATA
        vm_block_match_value_gs; /* explicit GS:0x6762 alias */
extern volatile cb_u16 CB_GAME_DATA vm_blood_history_ring_index; /* GS:0x6744 */
extern volatile cb_u16 CB_FAR * CB_GAME_DATA
        vm_blood_history_words; /* GS:0x6746 */
extern volatile cb_u16 vm_presentation_reg_6770; /* GS:0x6770 */
extern volatile cb_u16 CB_GAME_DATA
        vm_presentation_reg_6770_gs; /* explicit GS:0x6770 alias */
extern volatile cb_u16 CB_GAME_DATA vm_program_counter; /* GS:0x6772 */
extern volatile cb_u16 CB_GAME_DATA vm_parent_program_counter; /* GS:0x6774 */
extern volatile cb_u16 CB_GAME_DATA vm_pc_saved; /* GS:0x6776 */
extern volatile cb_u16 CB_GAME_DATA
        vm_post_update_record_offset; /* GS:0x6798 */
extern volatile cb_u16 CB_GAME_DATA
        vm_cd_replacement_kind_gs; /* GS:0x6792 */
extern volatile cb_u16 CB_GAME_DATA
        vm_cd_replacement_related_gs; /* GS:0x6794 */
extern volatile cb_u16 vm_text_loop_target;  /* GS:0x6778 */
extern volatile cb_u16 CB_GAME_DATA
        vm_text_loop_target_gs;              /* explicit GS:0x6778 alias */
extern volatile cb_u16 CB_GAME_DATA vm_resume_cursor; /* GS:0x677A */
extern volatile cb_u16 CB_GAME_DATA vm_branch_a; /* GS:0x6782 */
extern volatile cb_u16 CB_GAME_DATA vm_branch_b; /* GS:0x6784 */
extern volatile cb_u16 vm_presentation_owner_offset; /* DS:0x679A */
extern volatile cb_u16 vm_named_vbio_object; /* DS:0x679C */
extern cb_u8 CB_NEAR * volatile vm_text_selector_bytes; /* GS:0x677C */
extern cb_u8 CB_NEAR * volatile CB_GAME_DATA
        vm_text_selector_bytes_gs;           /* explicit GS:0x677C alias */
extern volatile cb_u8 vm_skip_count;         /* GS:0x67AB */
extern volatile cb_u8 CB_GAME_DATA
        vm_skip_count_gs;                    /* explicit GS:0x67AB alias */
extern volatile cb_u16 vm_active_line;       /* GS:0x6788 */
extern volatile cb_u16 CB_GAME_DATA
        vm_active_line_gs; /* explicit GS:0x6788 alias */
extern volatile cb_u16 vm_displayed_line;    /* DS:0x678A */
extern volatile cb_u16 vm_record_resource_handle; /* DS:0x6716 */
extern volatile cb_u16 vm_resource_profile_index; /* DS:0x677E */
extern volatile cb_u16 vm_primary_c4_record; /* DS:0x675E */
extern volatile cb_u16 CB_GAME_DATA
        vm_primary_c4_record_gs; /* explicit GS:0x675E alias */
extern volatile cb_u16 vm_named_scruter_jo_object; /* DS:0x6760 */
extern volatile cb_i16 vm_script_profile_request; /* GS:0x6780 */
extern volatile cb_u8 vm_execution_enabled; /* DS:0x67A8; runtime DS=GS */
extern volatile cb_u8 vm_presentation_request_flags; /* GS:0x67AA */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_request_flags_gs; /* explicit GS:0x67AA alias */
extern volatile cb_u8 vm_presentation_active; /* GS:0x67AC */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_active_gs; /* explicit GS:0x67AC alias */
extern volatile cb_u8 vm_word_choice_active; /* DS:0x27D7 */
extern volatile cb_u8 CB_GAME_DATA
        vm_word_choice_active_gs; /* explicit GS:0x27D7 alias */
extern volatile cb_u8 vm_query_mode;         /* GS:0x67AD */
extern volatile cb_u8 CB_GAME_DATA
        vm_query_mode_gs;                    /* explicit GS:0x67AD alias */
extern volatile cb_u16 vm_query_mode_word;   /* GS:0x67AD; includes 0x67AE */
extern volatile cb_u8 vm_presentation_defer_a; /* GS:0x67B0 */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_defer_a_gs;          /* explicit GS:0x67B0 alias */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */
extern volatile cb_u8 CB_GAME_DATA
        vm_resume_state_gs;                  /* explicit GS:0x67B1 alias */
extern volatile cb_u8 vm_block_scan_flags;   /* GS:0x67B2 */
extern volatile cb_u8 CB_GAME_DATA
        vm_block_scan_flags_gs;              /* explicit GS:0x67B2 alias */
extern volatile cb_u8 vm_yield_flag;         /* GS:0x67B4 */
extern volatile cb_u8 CB_GAME_DATA
        vm_yield_flag_gs;                    /* explicit GS:0x67B4 alias */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_pair_write_disabled; /* GS:0x67B6 */
extern volatile cb_u8 CB_GAME_DATA vm_presentation_start_lock; /* GS:0x67B7 */
extern volatile cb_u8 vm_text_word_list_mode; /* GS:0x67B9 */
extern volatile cb_u8 CB_GAME_DATA
        vm_text_word_list_mode_gs;           /* explicit GS:0x67B9 alias */
extern volatile cb_u8 vm_presentation_text_wait; /* GS:0x67BA */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_text_wait_gs; /* explicit GS:0x67BA alias */
extern volatile cb_u8 vm_presentation_word_choice_phase; /* DS:0x67BA alias */
extern volatile cb_u8 vm_dialogue_hold_complete; /* GS:0x67BB */
extern volatile cb_u8 vm_presentation_hold_ready; /* GS:0x67BC */
extern volatile cb_u8 CB_GAME_DATA
        vm_dialogue_hold_complete_gs; /* explicit GS:0x67BB alias */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_hold_ready_gs; /* explicit GS:0x67BC alias */
extern volatile cb_u8 vm_finale_requested;   /* GS:0x67BD */
extern volatile cb_u8 CB_GAME_DATA
        vm_finale_requested_gs;              /* explicit GS:0x67BD alias */
extern volatile cb_u16 vm_presentation_word_buffer[]; /* SS:0x67F8 here; SS=GS */
extern volatile cb_u16 vm_presentation_selected_word; /* DS:0x6796 */
extern volatile cb_u16 CB_GAME_DATA
        vm_presentation_word_buffer_gs[]; /* explicit GS:0x67F8 alias */
extern volatile cb_u16 vm_branch_stack_top;  /* GS:0x6884 */
extern volatile cb_u16 CB_GAME_DATA
        vm_branch_stack_top_gs;              /* explicit GS:0x6884 alias */
extern volatile cb_u16 vm_profile_cursor;     /* GS:0x6730 */
extern volatile cb_u16 vm_subtitle_wrap_marker; /* GS:0x6732 */
extern volatile cb_u16 vm_profile_record_word; /* GS:0x6734 */
extern volatile cb_u16 vm_c1_related_operand; /* GS:0x6736 */
extern volatile cb_u16 CB_GAME_DATA
        vm_c1_related_operand_gs;             /* explicit GS:0x6736 alias */
extern volatile cb_u16 vm_profile_word_6766;  /* GS:0x6766 */
extern volatile cb_u16 vm_profile_word_676e;  /* GS:0x676E */
extern volatile cb_u16 vm_profile_word_6786;  /* GS:0x6786 */
extern volatile cb_u16 vm_profile_word_67a0;  /* GS:0x67A0 */
extern volatile cb_u16 vm_profile_word_67a2;  /* GS:0x67A2 */
extern volatile cb_u8 vm_query_auxiliary;     /* GS:0x67AE */
extern volatile cb_u8 vm_profile_flag_67af;   /* GS:0x67AF */
extern volatile cb_u8 CB_GAME_DATA
        vm_presentation_related_flag20;       /* GS:0x67AF */
extern volatile bloodprg_vm_ui_state CB_GAME_DATA
        vm_ui_state_gs;                       /* explicit GS:0x2793 alias */
extern volatile cb_u16 CB_GAME_DATA
        vm_presentation_status_word;          /* GS:0x0A32 */
extern volatile cb_u16 CB_GAME_DATA
        vm_presentation_input_gate;           /* GS:0x2A19 */
extern volatile cb_u16 CB_GAME_DATA
        vm_deferred_record_type_gs;           /* GS:0x6768 */
extern volatile cb_u16 CB_GAME_DATA
        vm_deferred_record_related_gs;        /* GS:0x676A */
extern volatile cb_u16 CB_GAME_DATA
        vm_deferred_record_value_gs;          /* GS:0x676C */
extern volatile cb_u16 CB_GAME_DATA
        vm_presentation_owner_offset_gs;      /* explicit GS:0x679A alias */
extern volatile cb_u8 CB_GAME_DATA
        vm_c2_presentation_gate_gs;           /* explicit GS:0x1FB2 alias */
/* The shipped runtime has SS=GS; handlers preserve the floating script DS. */
extern bloodprg_vm_opcode_handler CB_NEAR *CB_GAME_DATA
        vm_opcode_handlers[];                /* GS/SS:0x6EB0 */
extern volatile cb_u16 vm_state_words[];     /* SS:0x6ADE here; SS=GS at runtime */
extern volatile cb_u16 CB_GAME_DATA
        vm_state_words_gs[];                 /* explicit GS/SS:0x6ADE alias */
/* Six rotating DESCRIPT sequence names; SS=GS at runtime. */
extern volatile char vm_record_string_slots[][16]; /* SS:0x6CDE */
extern volatile char CB_GAME_DATA vm_scene_name_buffer[]; /* ES=GS:0x209E */
extern volatile cb_u16
        vm_special_slots[BLOODPRG_VM_SPECIAL_SLOT_COUNT]; /* SS:0x6D3E; SS=DS */
extern const cb_i8 CB_GAME_DATA
        vm_field_offset_table_gs[];          /* explicit GS:0x6D60 alias */

extern const char CB_GAME_DATA vm_builtin_name_blood[];      /* DS:0x67BE */
extern const char CB_GAME_DATA vm_builtin_name_orxx[];       /* DS:0x67C4 */
extern const char CB_GAME_DATA vm_builtin_name_honk[];       /* DS:0x67C9 */
extern const char CB_GAME_DATA vm_builtin_name_menu[];       /* DS:0x67CE */
extern const char CB_GAME_DATA vm_builtin_name_arche[];      /* DS:0x67D3 */
extern const char CB_GAME_DATA vm_builtin_name_ark[];        /* DS:0x67E1 */
extern const char CB_GAME_DATA vm_builtin_name_scruter_jo[]; /* DS:0x67E5 */
extern const char CB_GAME_DATA vm_builtin_name_vbio[];       /* DS:0x67F0 */

void CB_FAR dlg_line_id_scene_dispatch(
        cb_u16 link_target_offset); /* 0x009D10 */
void CB_NEAR presentation_line_zero_run(
        cb_u16 link_target_offset); /* 0x001EC1 */
void CB_NEAR presentation_line_one_stream_run(
        cb_u16 link_target_offset); /* 0x001F10 */
#if defined(__WATCOMC__)
/* These modal loops call register-oriented graphics helpers.  The original
 * callers reload their live values after return; expose the same boundary so
 * Watcom does not hoist later SI/DI/ES arguments across either call. */
#pragma aux presentation_line_zero_run parm [ax] \
        modify exact [ax bx cx dx si di es]
#pragma aux presentation_line_one_stream_run parm [ax] \
        modify exact [ax bx cx dx si di es]
#endif
void CB_SAVE_REGS CB_FAR dlg_menu_words_inline_reveal_step(void); /* 0x0072A8 */
void CB_FAR presentation_ready_gate(void); /* 0x008963 */

#define BLOODPRG_VM_DIRECTORY_ACTIVE_KIND 0x0001u
#define BLOODPRG_VM_OBJECT_IN_PLAY_FLAG 0x02u
#define BLOODPRG_VM_OBJECT_ACCESS_MASK 0x0118u
#define BLOODPRG_VM_RECORD_C4 0x00c4u
#define BLOODPRG_VM_RECIPROCAL_SELECTOR 0x0013u

typedef struct bloodprg_vm_directory_entry {
    char name[16];
    cb_u16 object_offset;
    cb_u16 entry_kind;
} bloodprg_vm_directory_entry;

typedef const volatile bloodprg_vm_directory_entry CB_FAR *
        bloodprg_vm_directory_ptr;

#pragma pack(1)
typedef struct bloodprg_vm_patch_record {
    cb_u16 target_offset;
    cb_u8 value;
} bloodprg_vm_patch_record;
#pragma pack()

typedef char bloodprg_vm_patch_record_size_must_be_3[
        sizeof(bloodprg_vm_patch_record) == 3 ? 1 : -1];

typedef struct bloodprg_vm_object_header {
    cb_u16 kind;
    cb_u8 flags;
} bloodprg_vm_object_header;

typedef struct bloodprg_vm_scan_object {
    cb_u16 kind;
    cb_u16 flags;
    cb_u8 name[1];
} bloodprg_vm_scan_object;

typedef struct bloodprg_vm_state_record {
    cb_u16 kind;
    cb_u16 state;
} bloodprg_vm_state_record;

typedef struct bloodprg_vm_object_record {
    cb_u16 kind;
    cb_u8 flags;
    cb_u8 reserved_03[17];
    cb_u8 access_count;
} bloodprg_vm_object_record;

typedef struct bloodprg_vm_record_triple {
    cb_u16 kind;
    cb_u16 related;
    cb_u16 value;
} bloodprg_vm_record_triple;

typedef struct bloodprg_vm_opcode_descriptor {
    cb_u8 mode_zero_length;
    cb_i8 mode_one_length_or_control;
} bloodprg_vm_opcode_descriptor;

typedef char bloodprg_vm_opcode_descriptor_size_must_be_2[
        sizeof(bloodprg_vm_opcode_descriptor) == 2 ? 1 : -1];

/* The binary reads this through SS:BP; the shipped runtime has SS=GS.
 * It is the 0x60-byte table immediately after GS/DS:0x6EB0, at
 * DS/SS:0x6F18 (BLOODPRG.EXE file offset 0x14338). */
extern const bloodprg_vm_opcode_descriptor CB_GAME_DATA
        vm_opcode_descriptors[BLOODPRG_VM_OPCODE_WINDOW_COUNT];

typedef struct bloodprg_value_node {
    cb_u16 value;
    cb_u16 next_offset;
    cb_u8 payload[1];
} bloodprg_value_node;

typedef struct bloodprg_dic_lookup_result {
    cb_u16 object_offset;
    int matched;
} bloodprg_dic_lookup_result;

extern bloodprg_vm_directory_ptr vm_record_directory; /* DS:0x672C; runtime DS=GS */
extern bloodprg_vm_directory_ptr CB_GAME_DATA
        vm_record_directory_gs; /* explicit GS:0x672C alias */
extern const volatile bloodprg_vm_directory_entry CB_GAME_DATA
        vm_default_record_directory[]; /* GS:0x6F80 */
/* GS writer at 0x00604E, DS reader at 0x00721A; runtime DS=GS. */
extern volatile cb_u16 vm_active_object_offsets[]; /* 0x6A16 */
extern volatile cb_u16 CB_GAME_DATA
        vm_active_object_offsets_gs[]; /* explicit GS:0x6A16 alias */
/* SS:0x2AD3 in the binary; runtime SS=DS makes this ordinary near data. */
extern volatile cb_u16 vm_nav_chart_object_offsets[];

#if defined(__WATCOMC__)
#endif

int CB_FAR string_compare(const volatile char CB_FAR *left,
        const volatile char CB_FAR *right); /* 0x0025A4 */
void CB_NEAR object_heap_access(void);       /* 0x00149B */
void CB_SAVE_REGS CB_NEAR active_object_list_build(void); /* 0x00604E */
cb_u16 CB_FAR nav_chart_list_build(void);    /* 0x00721A */
const cb_u8 CB_NEAR *CB_NEAR value_scan_match(cb_u16 value,
        const bloodprg_value_node CB_FAR *node); /* 0x00577A */
cb_u16 CB_NEAR vm_patch_stream_apply(cb_u16 byte_count); /* 0x001D74 */
cb_u16 CB_NEAR vm_patch_stream_build(void);  /* 0x001D94 */
cb_i16 CB_SAVE_REGS CB_FAR vm_resource_profile_select(
        cb_u16 profile);                                  /* 0x0053A0 */
void CB_FAR vm_record_state_proc(void);       /* 0x00555B */
cb_i16 CB_FAR vm_run_wrapper(void);           /* 0x0055A4 */
#if defined(__WATCOMC__)
#endif
void CB_NEAR vm_op_a3_collect(void);             /* 0x005AFD */
cb_i16 CB_NEAR vm_script_block_scan(
        bloodprg_vm_image_ptr script_bytes);     /* 0x0056A6 */
void CB_NEAR vm_control_flow(
        const volatile bloodprg_vm_object_header CB_FAR *object,
        cb_u16 code_offset);                     /* 0x0056FE */
cb_u16 CB_NEAR vm_flag_test_67b1(void);           /* 0x005791 */
void CB_NEAR presentation_scan(void);              /* 0x005816 */
void CB_NEAR record_c1_ship3d_action(
        volatile bloodprg_vm_scan_object CB_FAR *object,
        volatile bloodprg_vm_record_triple CB_FAR *record); /* 0x005B38 */
void CB_SAVE_REGS CB_NEAR vm_state_processor(void); /* 0x005A74 */
int CB_NEAR vm_special_slot_remove(cb_u16 owner); /* 0x005FD8 */
int CB_NEAR vm_special_slot_insert(cb_u16 owner); /* 0x005FF6 */
int CB_NEAR vm_field_offset(cb_u16 selector, cb_u16 kind_mask); /* 0x006023 */
/* DS is the segment half of this far-pointer register contract. */
bloodprg_vm_image_ptr CB_NEAR vm_token_advance(
        bloodprg_vm_image_ptr script_bytes); /* 0x0062B6 */
cb_u16 CB_NEAR vm_cod_scan(cb_u16 object_offset); /* 0x00739B */
cb_u16 CB_NEAR vm_record_lookup_by_threshold(cb_u16 threshold); /* 0x006034 */
const cb_u8 CB_NEAR *CB_NEAR vm_token_special(cb_u16 terminator,
        const cb_u8 CB_NEAR *script_bytes); /* 0x006293 */
int CB_NEAR vm_condition_5(cb_u16 flags,
        const volatile cb_u8 CB_FAR *record,
        const cb_u8 *script_bytes);          /* 0x006339 */
bloodprg_dic_lookup_result CB_NEAR dic_word_lookup(cb_u16 dictionary_offset); /* 0x006433 */
cb_u16 CB_NEAR vm_branch_fail(void);         /* 0x006462 */
void CB_NEAR scan_zero_word(const cb_i16 CB_NEAR *script_words); /* 0x00647B */
const cb_u8 CB_NEAR *CB_NEAR vm_op_ce_cond_branch(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006494 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_d0_cond_branch(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0064A0 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_d1_cond_branch(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0064AC */
const cb_i8 CB_NEAR *CB_NEAR vm_op_d2_script_profile_request(
    const cb_i8 CB_NEAR *script_bytes);       /* 0x0064B8 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cf_clear_state(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0064C0 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cc_set_record_byte(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0064CE */
const cb_u16 CB_NEAR *CB_NEAR vm_op_ca_compare_var(
    const cb_u16 CB_NEAR *script_words);      /* 0x0064E5 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cb_compare_byte(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006510 */
const cb_u16 CB_NEAR *CB_NEAR vm_op_a0_push(
    const cb_u16 CB_NEAR *script_words);      /* 0x006559 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a1_pop(
    const cb_u8 CB_NEAR *script_bytes);        /* 0x006572 */
const cb_u16 CB_NEAR *CB_NEAR vm_op_a2_cond_call(
    const cb_u16 CB_NEAR *script_words);      /* 0x006588 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a3_block(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006596 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a4_jump(
    const cb_u16 CB_NEAR *script_words);      /* 0x0065DB */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a5_cond_state_array(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0065EB */
cb_u8 CB_NEAR *CB_NEAR vm_op_a6_text(
    cb_u8 CB_NEAR *script_bytes);             /* 0x00660C */
cb_u16 CB_NEAR strlen_b(const char CB_FAR *text); /* 0x0067A7 */
const cb_u16 CB_NEAR *CB_NEAR vm_op_a7_set_if_presentation(
    const cb_u16 CB_NEAR *script_words);      /* 0x0067BA */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a8_load_string(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0067C8 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_a9_cond_jump(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006830 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_ab_poke_byte(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x00684C */
const cb_u8 CB_NEAR *CB_NEAR vm_op_aa_yield(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006855 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_ac_yield(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x00685C */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_state_marker(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006863 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_ae_b0_state(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006902 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_record_wildcard(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006946 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cd_state_gated(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0069C7 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_b7_record_op(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006AA7 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_b8_record_readwrite(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006B06 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c1_record_state(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006B4C */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c2_record_full(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006E34 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c3_state_record(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006EEE */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c4_actor(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006C7E */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c5_record_match(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006D18 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c6_record_match(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006D80 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c7_record_match(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006DCF */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c8_record_match(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006F62 */
int CB_FAR vm_c2_descript_lookup(
    const volatile cb_u8 CB_FAR *record_name); /* 0x007409 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_c9_clear_record_full(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006FB9 */
cb_u16 CB_NEAR presentation_mode_bits_update(void); /* 0x009510 */
void CB_FAR presentation_update_1fb2(void); /* 0x009F53 */

#if defined(__WATCOMC__)
#pragma aux vm_token_special parm [ax] [si] value [si] modify exact [si]
#endif

#endif
