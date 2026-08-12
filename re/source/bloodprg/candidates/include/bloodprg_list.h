#ifndef BLOODPRG_LIST_H
#define BLOODPRG_LIST_H

#include "bloodprg_common.h"

extern volatile cb_u16 list_d8c_base_segment;      /* GS:0x0A7E */
extern volatile cb_u16 list_d8c_reserved_file_handle; /* game data:0x0A86 */
extern volatile cb_u8 state_flag_b17;              /* GS:0x0B17 */
extern volatile cb_u16 list_d8c_file_handle;       /* game data:0x0D5B */
extern volatile cb_u8 list_d8c_state_byte;         /* game data:0x0D5F */
extern volatile cb_u16 list_d8c_read_wrap_index;   /* game data:0x0D60 */
extern volatile cb_u16 list_d8c_wrap_count;        /* GS:0x0D62 */
extern volatile cb_u16 list_d8c_read_wrap_limit;   /* DS:0x0D64 */
extern volatile cb_u16 list_d8c_secondary_wrap_limit; /* DS:0x0D66 */
extern volatile cb_u16 list_d8c_head_offset;       /* GS:0x0D8C */
extern volatile cb_u16 list_d8c_head_segment;      /* GS:0x0D8E */
extern volatile cb_u16 list_d8c_tail_offset;       /* GS:0x0D90 */
extern volatile cb_u16 list_d8c_tail_segment;      /* GS:0x0D92 */
extern volatile cb_u16 CB_FAR *list_d8c_tail_pointer; /* DS:0x0D90 */
extern volatile cb_u16 list_d8c_active_offset;     /* GS:0x0D96 */
extern volatile cb_u16 list_d8c_wrap_limit;        /* GS:0x0D98 */
extern volatile cb_u16 list_d8c_byte_count;        /* game data:0x0D9A */
extern cb_u16 list_d8c_iteration_count;            /* GS:0x0DA0 */
extern cb_u16 list_d8c_entry_metric;               /* game data:0x0DAF */
extern volatile cb_u16 list_d8c_buffer_end_offset; /* GS:0x5233 */
extern volatile cb_u16 list_d8c_sequence_index;    /* DS:0x131C */
extern volatile cb_u8 CB_FAR list_d8c_buffer[];    /* segment at 0x0A7E */

void CB_NEAR close_file_d5b(void);              /* 0x00A141 */
volatile cb_u8 CB_FAR *CB_NEAR resource_palette_blocks_apply(
        volatile cb_u8 CB_FAR *stream);          /* 0x00A0C3 */
void CB_NEAR presentation_queue_finish(void);   /* 0x00A2DD */
int CB_NEAR list_d8c_read(cb_u16 *entry_extent,
        cb_u16 *cursor_offset);                  /* 0x00A622 */
int CB_NEAR ems_paged_read(cb_u16 byte_count);   /* 0x00A664 */
void CB_NEAR queue_d8c_wrap(cb_u16 byte_count, cb_u16 cursor); /* 0x00A38E */
int CB_NEAR queue_d8c_has_room(cb_u16 byte_count); /* 0x00A3AD */
void CB_NEAR queue_d8c_consume(void);            /* 0x00A3D0 */
int CB_NEAR list_d8c_state_le_one(void);         /* 0x00A40B */
int CB_NEAR flag_test_b17(void);                  /* 0x00A634 */
void CB_NEAR queue_d8c_enqueue(cb_u16 byte_count); /* 0x00A734 */
void CB_NEAR list_d8c_bounds_init(void);        /* 0x00A73E */
void CB_NEAR list_d8c_wrap_bounds_reset(void);  /* 0x00A744 */
void CB_FAR list_d8c_init(void);                /* 0x00A757 */

#endif
