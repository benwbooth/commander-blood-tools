#ifndef BLOODPRG_VM_H
#define BLOODPRG_VM_H

typedef unsigned char cb_u8;
typedef unsigned int cb_u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define CB_NEAR near
#else
#define CB_NEAR
#endif

extern volatile cb_u8 vm_sequence_active;    /* GS:0x252A */
extern volatile cb_u8 vm_scene_gate;         /* GS:0x274F */
extern volatile cb_u8 vm_ui_flags;           /* GS:0x2793 */
extern volatile cb_u16 vm_resume_value;      /* GS:0x6764 */
extern volatile cb_u8 vm_resume_state;       /* GS:0x67B1 */

void CB_NEAR vm_branch_fail(void);           /* 0x006462 */

#endif
