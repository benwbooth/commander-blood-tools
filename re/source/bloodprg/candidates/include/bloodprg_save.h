#ifndef BLOODPRG_SAVE_H
#define BLOODPRG_SAVE_H

#include "bloodprg_common.h"

#define BLOODPRG_SAVE_SLOT_NAME_BYTES 16u
#define BLOODPRG_SAVE_SLOT_NAME_LIMIT 14u

extern volatile cb_u16 save_slot_name_length;       /* DS:0x272E */
extern volatile cb_u16 save_slot_selected_index;    /* DS:0x2732 */
extern volatile cb_u8 CB_NEAR * volatile
        save_slot_active_name;                      /* DS:0x2734 */
extern volatile cb_u8 save_slot_edit_buffer[16];    /* DS:0x273B */
extern volatile cb_u16 save_slot_row_x;             /* DS:0x2AAB */
extern volatile cb_u16 save_slot_row_width;         /* DS:0x2AAF */

int CB_NEAR save_slot_name_edit_step(void);         /* 0x001DD8 */

#endif
