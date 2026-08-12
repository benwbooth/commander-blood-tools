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
extern volatile char vm_load_string_buffer[]; /* GS:0x2120 */
extern volatile cb_u8 vm_dialog_gate_0b3b;   /* GS:0x0B3B */
extern volatile cb_u8 vm_c2_presentation_gate; /* GS:0x1FB2 */
extern volatile cb_u16 vm_presentation_actor_record; /* GS:0x1FA3 */
extern const char CB_FAR *vm_dic_words; /* GS:0x6728 */
extern volatile cb_u8 CB_FAR *vm_record_base; /* GS:0x6724 */
extern volatile cb_u8 CB_FAR *vm_script_image; /* GS:0x671C */
extern volatile cb_u16 vm_branch_stack[];    /* GS:0x6820 */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764 */
extern volatile cb_u16 vm_arche_record_offset; /* GS:0x6752 */
extern volatile cb_u16 vm_block_match_value; /* GS:0x6762 */
extern volatile cb_u16 vm_blood_history_ring_index; /* GS:0x6744 */
extern volatile cb_u16 CB_FAR *vm_blood_history_words; /* GS:0x6746 */
extern volatile cb_u16 vm_presentation_reg_6770; /* GS:0x6770 */
extern volatile cb_u16 vm_active_line;       /* GS:0x6788 */
extern volatile cb_i16 vm_script_profile_request; /* GS:0x6780 */
extern volatile cb_u8 vm_presentation_request_flags; /* GS:0x67AA */
extern volatile cb_u8 vm_presentation_active; /* GS:0x67AC */
extern volatile cb_u8 vm_query_mode;         /* GS:0x67AD */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */
extern volatile cb_u8 vm_block_scan_flags;   /* GS:0x67B2 */
extern volatile cb_u8 vm_yield_flag;         /* GS:0x67B4 */
extern volatile cb_u8 vm_text_word_list_mode; /* GS:0x67B9 */
extern volatile cb_u8 vm_finale_requested;   /* GS:0x67BD */
extern volatile cb_u16 vm_presentation_word_buffer[]; /* GS:0x67F8 */
extern volatile cb_u16 vm_branch_stack_top;  /* GS:0x6884 */
extern volatile cb_u16 vm_state_words[];     /* GS:0x6ADE */
extern volatile char vm_record_string_slots[][16]; /* GS:0x6CDE */
extern volatile cb_u16 vm_special_slots[16]; /* SS:0x6D3E in helpers; DS alias */
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
void CB_NEAR vm_token_special(const cb_u8 **script_bytes, cb_u16 terminator); /* 0x006293 */
int CB_NEAR vm_condition_5(cb_u16 flags,
        const volatile cb_u8 CB_FAR *record,
        const cb_u8 *script_bytes);          /* 0x006339 */
bloodprg_dic_lookup_result CB_NEAR dic_word_lookup(cb_u16 dictionary_offset); /* 0x006433 */
cb_u16 CB_NEAR vm_branch_fail(void);         /* 0x006462 */
void CB_NEAR vm_op_c9_clear_record_full(const cb_u8 **script_bytes); /* 0x006FB9 */
void CB_NEAR presentation_mode_bits_update(void); /* 0x009510 */
void CB_FAR presentation_update_1fb2(void); /* 0x009F53 */

#endif
