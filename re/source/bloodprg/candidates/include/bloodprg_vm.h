#ifndef BLOODPRG_VM_H
#define BLOODPRG_VM_H

#include "bloodprg_common.h"
#include "bloodprg_random.h"

typedef union bloodprg_vm_ui_state {
    cb_u16 word;
    struct {
        cb_u8 flags;
        cb_u8 auxiliary;
    } bytes;
} bloodprg_vm_ui_state;

#define vm_ui_flags (vm_ui_state.bytes.flags)

extern volatile cb_i16 vm_compare_word;      /* GS:0x0AA6 */
extern volatile cb_i8 vm_compare_pair_low;   /* GS:0x0AA8 */
extern volatile cb_i8 vm_compare_pair_high;  /* GS:0x0AAA */
extern volatile cb_u8 vm_sequence_active;    /* GS:0x252A */
extern volatile cb_u8 vm_ship_3d_depth_step; /* GS:0x2531 */
extern volatile cb_u16 vm_ship_active_flags; /* GS:0x24F3 */
extern volatile cb_u8 vm_ship_active_flags_low; /* game data:0x24F3 */
extern volatile cb_u8 vm_scene_gate;         /* GS:0x274F */
extern volatile bloodprg_vm_ui_state vm_ui_state; /* GS:0x2793 */
extern volatile cb_i16 vm_bridge_view_frame; /* GS:0x2795 */
extern volatile cb_u8 vm_bridge_redraw_pending; /* GS:0x27D8 */
extern volatile cb_u16 vm_operand_word_count; /* GS:0x27CF */
extern volatile cb_u8 vm_load_string_buffer[]; /* SS:0x2120 here; SS=GS at runtime */
extern volatile cb_u8 vm_dialog_gate_0b3b;   /* GS:0x0B3B */
extern volatile cb_u8 vm_text_mode_0cf9;     /* GS:0x0CF9 */
extern volatile cb_u8 vm_text_mode_0cfa;     /* GS:0x0CFA */
extern volatile cb_u8 vm_text_voice_trigger; /* GS:0x0CFB */
extern volatile char vm_text_buffer[];       /* GS:0x0E18 */
extern volatile cb_i16 vm_text_selector;     /* GS:0x1FAB */
extern volatile cb_u8 vm_text_menu_pending;  /* GS:0x1FB3 */
extern volatile cb_u8 vm_c2_presentation_gate; /* GS:0x1FB2 */
extern volatile cb_u16 vm_presentation_actor_record; /* GS:0x1FA3 */
extern volatile cb_u16 vm_text_menu_end;     /* GS:0x27D3 */
extern volatile cb_u16 vm_text_reveal_cursor; /* GS:0x5E58 */
extern volatile cb_u8 vm_text_display_active; /* GS:0x5E64 */
extern const char CB_FAR *vm_dic_words; /* GS:0x6728 */
extern volatile cb_u8 CB_FAR *vm_record_base; /* GS:0x6724 */
extern volatile cb_u8 CB_FAR *vm_script_image; /* GS:0x671C */
extern volatile cb_u16 vm_branch_stack[];    /* SS:0x6820; SS=GS at runtime */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764; SS alias in 0x6596 */
extern const cb_u16 CB_FAR * volatile vm_text_menu_words; /* GS:0x674A */
extern volatile cb_u16 vm_arche_record_offset; /* GS:0x6752 */
extern volatile cb_u16 vm_wildcard_ref_value; /* GS:0x674E */
extern volatile cb_u16 vm_block_match_value; /* GS:0x6762; SS alias in 0x6596 */
extern volatile cb_u16 vm_blood_history_ring_index; /* GS:0x6744 */
extern volatile cb_u16 CB_FAR *vm_blood_history_words; /* GS:0x6746 */
extern volatile cb_u16 vm_presentation_reg_6770; /* GS:0x6770 */
extern volatile cb_u16 vm_text_loop_target;  /* GS:0x6778 */
extern volatile cb_u16 vm_branch_a;          /* GS:0x6782 */
extern cb_u8 CB_NEAR * volatile vm_text_selector_bytes; /* GS:0x677C */
extern volatile cb_u8 vm_skip_count;         /* GS:0x67AB */
extern volatile cb_u16 vm_active_line;       /* GS:0x6788 */
extern volatile cb_i16 vm_script_profile_request; /* GS:0x6780 */
extern volatile cb_u8 vm_presentation_request_flags; /* GS:0x67AA */
extern volatile cb_u8 vm_presentation_active; /* GS:0x67AC */
extern volatile cb_u8 vm_query_mode;         /* GS:0x67AD */
extern volatile cb_u8 vm_presentation_defer_a; /* GS:0x67B0 */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */
extern volatile cb_u8 vm_block_scan_flags;   /* GS:0x67B2 */
extern volatile cb_u8 vm_yield_flag;         /* GS:0x67B4 */
extern volatile cb_u8 vm_text_word_list_mode; /* GS:0x67B9 */
extern volatile cb_u8 vm_presentation_hold_ready; /* GS:0x67BC */
extern volatile cb_u8 vm_finale_requested;   /* GS:0x67BD */
extern volatile cb_u16 vm_presentation_word_buffer[]; /* SS:0x67F8 here; SS=GS */
extern volatile cb_u16 vm_branch_stack_top;  /* GS:0x6884 */
extern volatile cb_u16 vm_state_words[];     /* SS:0x6ADE here; SS=GS at runtime */
extern volatile char vm_record_string_slots[][16]; /* SS:0x6CDE; SS=GS at runtime */
extern volatile cb_u16 vm_special_slots[16]; /* SS:0x6D3E in helpers; runtime SS=DS */
extern const cb_i8 CB_FAR vm_field_offset_table[]; /* GS:0x6D60 */

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

typedef struct bloodprg_vm_object_header {
    cb_u16 kind;
    cb_u8 flags;
} bloodprg_vm_object_header;

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

typedef struct bloodprg_value_node {
    cb_u16 value;
    const struct bloodprg_value_node CB_NEAR *next;
    cb_u8 payload[1];
} bloodprg_value_node;

typedef struct bloodprg_dic_lookup_result {
    cb_u16 object_offset;
    int matched;
} bloodprg_dic_lookup_result;

extern const volatile bloodprg_vm_directory_entry CB_FAR *vm_record_directory; /* GS:0x672C */
extern volatile cb_u16 vm_active_object_offsets[]; /* GS:0x6A16 */

#if defined(__WATCOMC__)
#pragma aux vm_special_slot_remove parm [ax] value [ax] modify exact [ax]
#pragma aux vm_special_slot_insert parm [ax] value [ax] modify exact [ax]
#pragma aux vm_field_offset parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux vm_record_lookup_by_threshold parm [ax] value [ax] modify exact [ax]
#pragma aux vm_token_special parm [ax] [si] value [si] modify exact [si]
#pragma aux vm_condition_5 parm [cx] [es di] [si] value [ax] modify exact [ax bx dx]
#pragma aux vm_branch_fail value [si] modify exact [ax si]
#pragma aux scan_zero_word parm [si] modify exact [ax]
#pragma aux vm_op_d2_script_profile_request parm [si] value [si] modify exact [ax si]
#pragma aux vm_op_ce_cond_branch modify exact [ax si]
#pragma aux vm_op_d0_cond_branch modify exact [ax si]
#pragma aux vm_op_d1_cond_branch modify exact [ax si]
#pragma aux vm_op_cc_set_record_byte parm [si] value [si] modify exact [ax si]
#pragma aux vm_op_ca_compare_var parm [si] value [si] modify exact [ax dx si]
#pragma aux vm_op_cb_compare_byte parm [si] value [si] modify exact [ax bx dx si]
#pragma aux vm_op_a0_push parm [si] value [si] modify exact [ax bp si]
#pragma aux vm_op_a1_pop value [ax] modify exact [ax]
#pragma aux vm_op_a2_cond_call parm [si] value [si] modify exact [ax si]
#pragma aux vm_op_a3_block parm [si] value [si] modify exact [ax bp dx si]
#pragma aux vm_op_a4_jump parm [si] value [si] modify exact [si]
#pragma aux vm_op_a5_cond_state_array parm [si] value [si] modify exact [ax bp si]
#pragma aux vm_op_a6_text parm [si] value [si] modify exact [ax bx cx dx si es]
#pragma aux strlen_b parm [es di] value [ax] modify exact [ax]
#pragma aux vm_op_a7_set_if_presentation parm [si] value [si] modify exact [ax si]
#pragma aux vm_op_a8_load_string parm [si] value [si] modify exact [ax bp si]
#pragma aux vm_op_a9_cond_jump parm [si] value [si] modify exact [ax si]
#pragma aux vm_op_ab_poke_byte parm [si] value [si] modify exact [ax bx si]
#pragma aux vm_op_shared_state_marker parm [si] value [si] modify exact [ax bx cx dx si es]
#pragma aux vm_op_shared_ae_b0_state parm [si] value [si] modify exact [ax bx dx si es]
#pragma aux vm_op_shared_record_wildcard parm [si] value [si] modify exact [ax bx cx dx si es]
#pragma aux vm_op_cd_state_gated parm [si] value [si] modify exact [ax bx cx dx si bp]
#pragma aux vm_c2_descript_lookup parm [es di] value [ax] modify exact [ax]
#endif

int CB_FAR string_compare(const volatile char CB_FAR *left,
        const volatile char CB_FAR *right); /* 0x0025A4 */
void CB_NEAR object_heap_access(void);       /* 0x00149B */
const cb_u8 CB_NEAR *CB_NEAR value_scan_match(cb_u16 value,
        const bloodprg_value_node CB_NEAR *node); /* 0x00577A */
void CB_NEAR vm_patch_stream_apply(cb_u16 byte_count); /* 0x001D74 */
int CB_NEAR vm_special_slot_remove(cb_u16 owner); /* 0x005FD8 */
int CB_NEAR vm_special_slot_insert(cb_u16 owner); /* 0x005FF6 */
int CB_NEAR vm_field_offset(cb_u16 selector, cb_u16 kind_mask); /* 0x006023 */
cb_u16 CB_NEAR vm_record_lookup_by_threshold(cb_u16 threshold); /* 0x006034 */
const cb_u8 CB_NEAR *CB_NEAR vm_token_special(cb_u16 terminator,
        const cb_u8 CB_NEAR *script_bytes); /* 0x006293 */
int CB_NEAR vm_condition_5(cb_u16 flags,
        const volatile cb_u8 CB_FAR *record,
        const cb_u8 *script_bytes);          /* 0x006339 */
bloodprg_dic_lookup_result CB_NEAR dic_word_lookup(cb_u16 dictionary_offset); /* 0x006433 */
cb_u16 CB_NEAR vm_branch_fail(void);         /* 0x006462 */
void CB_NEAR scan_zero_word(const cb_i16 CB_NEAR *script_words); /* 0x00647B */
void CB_NEAR vm_op_ce_cond_branch(void);      /* 0x006494 */
void CB_NEAR vm_op_d0_cond_branch(void);      /* 0x0064A0 */
void CB_NEAR vm_op_d1_cond_branch(void);      /* 0x0064AC */
const cb_i8 CB_NEAR *CB_NEAR vm_op_d2_script_profile_request(
    const cb_i8 CB_NEAR *script_bytes);       /* 0x0064B8 */
void CB_NEAR vm_op_cf_clear_state(void);      /* 0x0064C0 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cc_set_record_byte(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0064CE */
const cb_u16 CB_NEAR *CB_NEAR vm_op_ca_compare_var(
    const cb_u16 CB_NEAR *script_words);      /* 0x0064E5 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cb_compare_byte(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006510 */
const cb_u16 CB_NEAR *CB_NEAR vm_op_a0_push(
    const cb_u16 CB_NEAR *script_words);      /* 0x006559 */
cb_u16 CB_NEAR vm_op_a1_pop(void);            /* 0x006572 */
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
void CB_NEAR vm_op_aa_yield(void);            /* 0x006855 */
void CB_NEAR vm_op_ac_yield(void);            /* 0x00685C */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_state_marker(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006863 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_ae_b0_state(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006902 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_record_wildcard(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x006946 */
const cb_u8 CB_NEAR *CB_NEAR vm_op_cd_state_gated(
    const cb_u8 CB_NEAR *script_bytes);       /* 0x0069C7 */
int CB_FAR vm_c2_descript_lookup(
    const volatile cb_u8 CB_FAR *record_name); /* 0x007409 */
void CB_NEAR vm_op_c9_clear_record_full(const cb_u8 **script_bytes); /* 0x006FB9 */
void CB_NEAR presentation_mode_bits_update(void); /* 0x009510 */
void CB_FAR presentation_update_1fb2(void); /* 0x009F53 */

#endif
