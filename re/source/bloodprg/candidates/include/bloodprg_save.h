#ifndef BLOODPRG_SAVE_H
#define BLOODPRG_SAVE_H

#include "bloodprg_common.h"

#define BLOODPRG_SAVE_SLOT_NAME_BYTES 16u
#define BLOODPRG_SAVE_SLOT_NAME_LIMIT 14u
#define BLOODPRG_SAVE_SLOT_COUNT 10u

typedef struct bloodprg_save_slot {
    cb_u8 name[BLOODPRG_SAVE_SLOT_NAME_BYTES];
    cb_u8 filename[BLOODPRG_SAVE_SLOT_NAME_BYTES];
} bloodprg_save_slot;

typedef char bloodprg_save_slot_size_must_be_32[
        sizeof(bloodprg_save_slot) == 32 ? 1 : -1];

extern volatile cb_u16 save_slot_name_length;       /* DS:0x272E */
extern volatile cb_u16 save_slot_selected_index;    /* DS:0x2732 */
extern volatile cb_u8 CB_NEAR * volatile
        save_slot_active_name;                      /* DS:0x2734 */
extern volatile cb_u8 save_slot_edit_buffer[16];    /* DS:0x273B */
extern volatile cb_u16 save_slot_row_x;             /* DS:0x2AAB */
extern volatile cb_u16 save_slot_row_width;         /* DS:0x2AAF */
extern const cb_u8 save_slot_quick_name_source[8];  /* DS:0x0161 */
extern const char save_slot_directory_path[];       /* DS:0x00FC */
extern const cb_u16 save_slot_item_offsets[];       /* DS:0x25D7 */
extern volatile bloodprg_save_slot
        save_slot_records[BLOODPRG_SAVE_SLOT_COUNT]; /* DS:0x25ED */
extern volatile cb_u8 save_request_active;          /* DS:0x2736 */
extern volatile cb_u8 load_request_active;          /* DS:0x2737 */
extern volatile cb_u8 save_slot_menu_phase;         /* DS:0x2738 */
extern volatile cb_u8 quicksave_request_active;     /* DS:0x2739 */
extern volatile cb_u8 save_slot_transition_aux;     /* DS:0x0ADC */
extern volatile cb_u8 save_load_redraw_pending;     /* DS:0x27D9 */

int CB_NEAR save_slot_name_edit_step(void);         /* 0x001DD8 */
void CB_NEAR save_load_menu_step(void);             /* 0x001B4B */

#endif
