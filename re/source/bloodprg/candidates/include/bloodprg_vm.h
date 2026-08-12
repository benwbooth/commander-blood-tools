#ifndef BLOODPRG_VM_H
#define BLOODPRG_VM_H

#include "bloodprg_common.h"

extern volatile cb_i16 vm_compare_word;      /* GS:0x0AA6 */
extern volatile cb_i8 vm_compare_pair_low;   /* GS:0x0AA8 */
extern volatile cb_i8 vm_compare_pair_high;  /* GS:0x0AAA */
extern volatile cb_u8 vm_sequence_active;    /* GS:0x252A */
extern volatile cb_u16 vm_ship_active_flags; /* GS:0x24F3 */
extern volatile cb_u8 vm_scene_gate;         /* GS:0x274F */
extern volatile cb_u8 vm_ui_flags;           /* GS:0x2793 */
extern volatile cb_u16 vm_operand_word_count; /* GS:0x27CF */
extern volatile char vm_load_string_buffer[]; /* GS:0x2120 */
extern volatile cb_u8 vm_dialog_gate_0b3b;   /* GS:0x0B3B */
extern volatile cb_u8 vm_c2_presentation_gate; /* GS:0x1FB2 */
extern volatile cb_u16 vm_presentation_actor_record; /* GS:0x1FA3 */
extern volatile cb_u8 CB_FAR *vm_record_base; /* GS:0x6724 */
extern volatile cb_u16 vm_branch_stack[];    /* GS:0x6820 */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764 */
extern volatile cb_u8 CB_FAR *vm_secondary_record; /* GS:0x6752 */
extern volatile cb_u16 vm_presentation_reg_6770; /* GS:0x6770 */
extern volatile cb_u16 vm_active_line;       /* GS:0x6788 */
extern volatile cb_i16 vm_script_profile_request; /* GS:0x6780 */
extern volatile cb_u8 vm_presentation_request_flags; /* GS:0x67AA */
extern volatile cb_u8 vm_presentation_active; /* GS:0x67AC */
extern volatile cb_u8 vm_query_mode;         /* GS:0x67AD */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */
extern volatile cb_u8 vm_yield_flag;         /* GS:0x67B4 */
extern volatile cb_u8 vm_finale_requested;   /* GS:0x67BD */
extern volatile cb_u16 vm_branch_stack_top;  /* GS:0x6884 */
extern volatile cb_u16 vm_state_words[];     /* GS:0x6ADE */
extern volatile char vm_record_string_slots[][16]; /* GS:0x6CDE */
extern const cb_i8 CB_FAR vm_field_offset_table[]; /* GS:0x6D60 */

int CB_FAR blood_prng_next(cb_u16 modulus);  /* 0x002DE2 */
cb_u16 CB_NEAR vm_record_lookup_by_threshold(cb_u16 threshold); /* 0x006034 */
void CB_NEAR vm_branch_fail(void);           /* 0x006462 */

#endif
