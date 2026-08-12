#ifndef BLOODPRG_LIST_H
#define BLOODPRG_LIST_H

#include "bloodprg_common.h"

extern volatile cb_u16 list_d8c_base_segment;      /* GS:0x0A7E */
extern volatile cb_u8 state_flag_b17;              /* GS:0x0B17 */
extern volatile cb_u8 list_d8c_state_byte;         /* GS:0x0D5F */
extern volatile cb_u16 list_d8c_wrap_count;        /* GS:0x0D62 */
extern volatile cb_u16 list_d8c_head_offset;       /* GS:0x0D8C */
extern volatile cb_u16 list_d8c_head_segment;      /* GS:0x0D8E */
extern volatile cb_u16 list_d8c_tail_offset;       /* GS:0x0D90 */
extern volatile cb_u16 list_d8c_tail_segment;      /* GS:0x0D92 */
extern volatile cb_u16 list_d8c_active_offset;     /* GS:0x0D96 */
extern volatile cb_u16 list_d8c_wrap_limit;        /* GS:0x0D98 */
extern volatile cb_u16 list_d8c_byte_count;        /* GS:0x0D9A */
extern volatile cb_u16 list_d8c_iteration_count;   /* GS:0x0DA0 */
extern volatile cb_u16 list_d8c_buffer_end_offset; /* GS:0x5233 */

void CB_NEAR close_file_d5b(void);              /* 0x00A141 */
void CB_NEAR presentation_queue_finish(void);   /* 0x00A2DD */

#endif
