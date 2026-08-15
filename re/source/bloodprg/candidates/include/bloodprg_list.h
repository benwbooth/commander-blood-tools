#ifndef BLOODPRG_LIST_H
#define BLOODPRG_LIST_H

#include "bloodprg_common.h"

typedef struct bloodprg_resource_decode_result {
    const volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *destination;
} bloodprg_resource_decode_result;

typedef char bloodprg_resource_decode_result_size_must_be_8[
        sizeof(bloodprg_resource_decode_result) == 8 ? 1 : -1];

extern volatile cb_u16 list_d8c_base_segment;      /* GS:0x0A7E */
extern volatile cb_u8 CB_GAME_DATA state_flag_b17; /* GS:0x0B17 */
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
extern volatile cb_u16 list_d8c_active_offset;     /* GS:0x0D94 */
extern volatile cb_u16 list_d8c_active_segment;    /* GS:0x0D96 */
extern volatile cb_u16 list_d8c_wrap_limit;        /* GS:0x0D98 */
extern volatile cb_u16 list_d8c_byte_count;        /* game data:0x0D9A */
extern volatile cb_u16 list_d8c_sound_offset;      /* game data:0x0D9C */
extern volatile cb_u16 list_d8c_palette_offset;    /* game data:0x0D9E */
extern cb_u16 list_d8c_iteration_count;            /* GS:0x0DA0 */
extern cb_u16 list_d8c_entry_metric;               /* game data:0x0DAF */
extern volatile cb_u16 list_d8c_buffer_end_offset; /* GS:0x5233 */
extern volatile cb_u16 list_d8c_sequence_index;    /* DS:0x131C */
extern volatile cb_u8 CB_FAR list_d8c_buffer[];    /* segment at 0x0A7E */
extern volatile cb_u16 list_d8c_default_entry_segment; /* GS:0x0ABE */
extern volatile cb_u16 list_d8c_alternate_entry_segment; /* GS:0x0DA8 */
extern volatile cb_u16 list_d8c_active_layout;     /* GS:0x0DA4 */
extern volatile cb_u16 list_d8c_active_row_mode;   /* GS:0x0DA6 */
extern volatile cb_u16 list_d8c_retired_segment;   /* GS:0x0DAA */
extern volatile cb_u8 list_d8c_rollover_state;     /* DS:0x0DAC */
extern volatile cb_u8 resource_frame_presented;    /* GS:0x0DB8 */
extern volatile cb_u8 resource_draw_via_back_buffer; /* GS:0x0DB9 */
extern volatile cb_u8 resource_decode_rectangular; /* GS:0x0DBA */
extern volatile cb_u8 resource_skip_back_buffer_present; /* GS:0x0DBB */
extern volatile cb_u8 resource_unclamped_row_count; /* GS:0x0DBD */
extern const cb_u8 presentation_unclamped_line_ids[9]; /* caller ES:0x0DBE */
extern volatile cb_u16 resource_vertical_offset;   /* GS:0x1FA7 */
extern volatile cb_u16 CB_GAME_DATA
        resource_vertical_offset_gs; /* explicit GS:0x1FA7 alias */
extern volatile cb_u16 timer_tick_count;          /* DS:0x0B29 */
extern volatile cb_u16 list_d8c_audio_phase;      /* DS:0x0C41 */
extern volatile cb_u8 list_d8c_tick_threshold;    /* DS:0x0D77 */
extern volatile cb_u16 list_d8c_previous_tick;    /* DS:0x0DA2 */
extern volatile cb_u16 CB_GAME_DATA resource_decode_mode; /* GS:0x0AA0 */

void CB_NEAR close_file_d5b(void);              /* 0x00A141 */
volatile cb_u8 CB_FAR *CB_NEAR resource_palette_blocks_apply(
        volatile cb_u8 CB_FAR *stream);          /* 0x00A0C3 */
int CB_NEAR list_d8c_activate_ready(void);      /* 0x00A20C */
int CB_NEAR list_d8c_advance_due(void);         /* 0x00A240 */
void CB_NEAR list_d8c_refill_with_rollover_latch(
        cb_u16 link_target_offset);              /* 0x00A1F3 */
void CB_NEAR ems_resource_flush(
        cb_u16 link_target_offset);              /* 0x00A1B4 */
void CB_NEAR resource_load_sequence(cb_u16 resource_id); /* 0x00A15F */
void CB_NEAR list_d8c_activate_entry(cb_u16 entry_extent,
        volatile cb_u16 CB_FAR *entry,
        cb_u16 storage_segment);                /* 0x00A552 */
cb_u16 CB_NEAR list_d8c_refill(cb_u16 link_target_offset); /* 0x00A2AB */
void CB_NEAR presentation_queue_finish(void);   /* 0x00A2DD */
int CB_NEAR list_d8c_read(cb_u16 *entry_extent,
        cb_u16 *cursor_offset);                  /* 0x00A622 */
int CB_NEAR banked_list_load(void);              /* 0x00A642 */
int CB_NEAR ems_paged_read(cb_u16 byte_count);   /* 0x00A664 */
void CB_NEAR queue_d8c_wrap(cb_u16 byte_count, cb_u16 cursor); /* 0x00A38E */
int CB_NEAR queue_d8c_has_room(cb_u16 byte_count); /* 0x00A3AD */
void CB_NEAR queue_d8c_consume(void);            /* 0x00A3D0 */
int CB_NEAR list_d8c_state_le_one(void);         /* 0x00A40B */
void CB_FAR list_d8c_active_present(void);       /* 0x00A41A */
cb_u8 CB_NEAR flag_test_b17(void);                /* 0x00A634 */
void CB_NEAR queue_d8c_enqueue(cb_u16 byte_count); /* 0x00A734 */
void CB_NEAR list_d8c_bounds_init(void);        /* 0x00A73E */
void CB_NEAR list_d8c_wrap_bounds_reset(void);  /* 0x00A744 */
void CB_FAR list_d8c_init(void);                /* 0x00A757 */
volatile cb_u8 CB_FAR *CB_NEAR list_d8c_palette_blocks_apply(void); /* 0x00A778 */
void CB_NEAR resource_payload_decode_ab(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination);       /* 0x00A867 */
void CB_NEAR resource_payload_decode_ad(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination);       /* 0x00A914 */
void CB_NEAR resource_payload_decode_rect(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *staging,
        volatile cb_u8 CB_FAR *framebuffer,
        cb_u16 vertical_offset,
        cb_u16 row_width,
        cb_u16 rows);                              /* 0x00AB25 */
void CB_NEAR resource_rect_blit(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *framebuffer,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 row_mode);                          /* 0x00A4ED */
const volatile cb_u8 CB_FAR *CB_NEAR resource_pair_lz_decode(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination,
        volatile cb_u8 CB_FAR *destination_end,
        cb_u8 literal_bias);                       /* 0x00AABC */
bloodprg_resource_decode_result CB_NEAR resource_payload_decode_dispatch(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination,
        cb_u16 alternate_destination_segment);     /* 0x00A82C */

#if defined(__WATCOMC__)
#pragma aux resource_palette_blocks_apply \
        parm [es si] value [es si] modify [si di]
#pragma aux list_d8c_palette_blocks_apply \
        value [es si] modify [si di]
#pragma aux flag_test_b17 modify exact [ax]
#pragma aux resource_pair_lz_decode value [ds bx] \
        modify exact [ax bx cx dx si di]
#endif

#endif
