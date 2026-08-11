#ifndef BLOODPRG_VM_H
#define BLOODPRG_VM_H

#include "bloodprg_common.h"

extern volatile cb_u8 vm_sequence_active;    /* GS:0x252A */
extern volatile cb_u8 vm_scene_gate;         /* GS:0x274F */
extern volatile cb_u8 vm_ui_flags;           /* GS:0x2793 */
extern volatile cb_u16 vm_operand_word_count; /* GS:0x27CF */
extern volatile cb_u16 vm_branch_stack[];    /* GS:0x6820 */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764 */
extern volatile cb_i16 vm_script_profile_request; /* GS:0x6780 */
extern volatile cb_u8 vm_query_mode;         /* GS:0x67AD */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */
extern volatile cb_u8 vm_yield_flag;         /* GS:0x67B4 */
extern volatile cb_u16 vm_branch_stack_top;  /* GS:0x6884 */
extern const cb_i8 CB_FAR vm_field_offset_table[]; /* GS:0x6D60 */

void CB_NEAR vm_branch_fail(void);           /* 0x006462 */

#endif
