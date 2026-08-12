#ifndef BLOODPRG_LIST_H
#define BLOODPRG_LIST_H

#include "bloodprg_common.h"

extern volatile cb_u16 list_d8c_base_segment;      /* GS:0x0A7E */
extern volatile cb_u16 list_d8c_head_offset;       /* GS:0x0D8C */
extern volatile cb_u16 list_d8c_head_segment;      /* GS:0x0D8E */
extern volatile cb_u16 list_d8c_tail_offset;       /* GS:0x0D90 */
extern volatile cb_u16 list_d8c_tail_segment;      /* GS:0x0D92 */
extern volatile cb_u16 list_d8c_active_offset;     /* GS:0x0D96 */
extern volatile cb_u16 list_d8c_wrap_limit;        /* GS:0x0D98 */
extern volatile cb_u16 list_d8c_byte_count;        /* GS:0x0D9A */
extern volatile cb_u16 list_d8c_iteration_count;   /* GS:0x0DA0 */
extern volatile cb_u16 list_d8c_buffer_end_offset; /* GS:0x5233 */

#endif
