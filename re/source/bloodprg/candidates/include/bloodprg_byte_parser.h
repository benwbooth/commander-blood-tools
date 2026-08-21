#ifndef BLOODPRG_BYTE_PARSER_H
#define BLOODPRG_BYTE_PARSER_H

#include "bloodprg_common.h"
#include "bloodprg_ems.h"

typedef volatile char CB_GAME_DATA *cb_game_char_ptr;
typedef volatile cb_u16 CB_GAME_DATA *cb_game_word_ptr;
typedef volatile cb_u8 CB_FAR *cb_far_u8_ptr;

extern volatile cb_u8 CB_GAME_DATA byte_parser_b16_flag; /* GS:0x0B16 */
extern volatile cb_u16 CB_GAME_DATA descript_directory_count; /* GS:0x0AAE */
extern volatile cb_u16 CB_GAME_DATA descript_record_length; /* GS:0x0AB0 */
extern volatile char CB_GAME_DATA descript_database_path[]; /* GS:0x0106 */
extern volatile char CB_GAME_DATA byte_parser_background_path[]; /* GS:0x0DC7 */
extern volatile char CB_GAME_DATA
        byte_parser_background_slots[][16]; /* GS:0x0DD7 */
extern volatile cb_u32 CB_GAME_DATA
        byte_parser_background_source_size; /* GS:0x0A92 */
extern char CB_GAME_DATA byte_parser_table_2460[]; /* ES:0x2460 */
extern char CB_GAME_DATA byte_parser_table_247a[]; /* ES:0x247A */
extern char CB_GAME_DATA byte_parser_line_name[];  /* ES:0x24C6 */
extern char CB_GAME_DATA byte_parser_text_20b8[];  /* ES:0x20B8 */
extern char CB_FS_DATA fs_resource_name_area[];       /* FS:0x0C74 */
extern char CB_GAME_DATA credit_text_buffer[];          /* ES:0x0E18 */
extern volatile cb_u8 CB_GAME_DATA credit_reveal_active; /* GS:0x5E64 */
extern volatile cb_u16 CB_GAME_DATA credit_reveal_timer; /* GS:0x5E58 */
extern volatile cb_u8 CB_GAME_DATA fs_name_area_dirty; /* GS:0x27E8 */
extern char CB_GAME_DATA music_voc_name_field[];       /* ES:0x0D30 */
extern volatile cb_u8 CB_GAME_DATA music_voc_name_unchanged; /* GS:0x0BA0 */
extern volatile cb_u8 CB_GAME_DATA music_voc_name_changed; /* GS:0x0BA1 */
extern volatile char CB_GAME_DATA byte_parser_snd_bank_path[]; /* GS:0x0D06 */
extern char CB_GAME_DATA byte_parser_snd_bank_name_field[]; /* ES:0x0D09 */
extern volatile cb_u16 CB_GAME_DATA byte_parser_ui_state; /* GS:0x2793 */
extern volatile cb_u16 CB_GAME_DATA byte_parser_index_word_1fd7; /* ES:0x1FD7 */
extern volatile char CB_GAME_DATA byte_parser_index_path_2137[]; /* GS:0x2137 */
extern volatile char CB_GAME_DATA byte_parser_index_text_213a[]; /* ES:0x213A */
extern volatile cb_far_u8_ptr CB_GAME_DATA byte_parser_back_buffer; /* GS:0x5229 */
extern volatile cb_u16 CB_GAME_DATA byte_parser_word_1fa5; /* GS:0x1FA5 */
extern volatile cb_game_char_ptr CB_GAME_DATA byte_parser_detail_cursor; /* GS:0x1FAD */
extern volatile cb_game_word_ptr CB_GAME_DATA byte_parser_asset_cursor; /* GS:0x1FAF */
extern volatile cb_game_char_ptr CB_GAME_DATA byte_parser_table_131a_cursor; /* GS:0x131A */
extern volatile cb_u8 CB_GAME_DATA byte_parser_table_131e_count; /* GS:0x131E */
extern volatile cb_i16 CB_GAME_DATA
        byte_parser_table_131c_visible_index; /* GS:0x131C */
extern volatile cb_game_char_ptr CB_GAME_DATA byte_parser_stream_0f18_cursor; /* GS:0x0F18 */
extern volatile cb_u8 CB_GAME_DATA
        byte_parser_stream_segment[]; /* GS-relative byte arena */
/* Ordinary DS aliases consumed by the presentation controller at 0x0079E5. */
extern volatile char CB_NEAR * volatile descript_centered_text_cursor; /* DS:0x0F18 */
extern volatile cb_u8 descript_centered_text_events[]; /* DS:0x0F1A */
extern volatile char CB_NEAR * volatile descript_text_record_cursor; /* DS:0x131A */
extern volatile cb_u8 descript_text_record_count; /* DS:0x131E */
extern volatile cb_u8 descript_text_records_remaining; /* DS:0x131F */
extern volatile char descript_text_record_table[][16]; /* DS:0x1320 */

void CB_NEAR byte_parser_op_01_mark_b16(void); /* 0x007542 */
void CB_NEAR byte_parser_op_02_mark_b16(void); /* 0x007549 */
void CB_NEAR byte_parser_op_0f_mark_b16(void); /* 0x007550 */
void CB_NEAR byte_parser_op_04_mark_b16(void); /* 0x007557 */
const cb_u8 CB_FAR *CB_NEAR index_lookup_dca(
    const cb_u8 CB_FAR *script_bytes); /* 0x00755E */
const cb_u8 CB_FAR *CB_NEAR credit_presenter_b_cryo(
    const cb_u8 CB_FAR *script_bytes); /* 0x007612 */
const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_20b8_printable(
    const cb_u8 CB_FAR *script_bytes); /* 0x007629 */
const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_24c6_printable(
    const cb_u8 CB_FAR *script_bytes); /* 0x00766F */
const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_2460_printable(
    const cb_u8 CB_FAR *script_bytes); /* 0x0076C0 */
const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_247a_printable(
    const cb_u8 CB_FAR *script_bytes); /* 0x0076D5 */
const cb_u8 CB_FAR *CB_NEAR byte_parser_snd_bank_name_load(
    const cb_u8 CB_FAR *script_bytes); /* 0x00763E */
const cb_u8 CB_FAR *CB_NEAR dlg_line_asset_table_fill(
    const cb_u8 CB_FAR *script_bytes); /* 0x007684 */
const cb_u16 CB_FAR *CB_NEAR byte_parser_store_word_1fa5(
    const cb_u16 CB_FAR *script_words); /* 0x0076BA */
const cb_u8 CB_FAR *CB_NEAR index_lookup_1fd7(
    const cb_u8 CB_FAR *script_bytes); /* 0x0076EA */
const cb_u8 CB_FAR *CB_NEAR byte_parser_copy_131a_entry(
    const cb_u8 CB_FAR *script_bytes); /* 0x007754 */
const cb_u8 CB_FAR *CB_NEAR byte_parser_stream_0f18_append(
    const cb_u8 CB_FAR *script_bytes); /* 0x007776 */
const cb_u8 CB_FAR *CB_NEAR fs_name_area_read(
    const cb_u8 CB_FAR *script_bytes); /* 0x007788 */
const cb_u8 CB_FAR *CB_NEAR music_voc_name_patcher(
    const cb_u8 CB_FAR *script_bytes); /* 0x0077A9 */

void CB_FAR resource_file_load_to_ems(
        volatile char CB_FAR *path); /* 0x01CE:0712 */
void CB_FAR resource_file_load_to_xms(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *staging_buffer); /* 0x01CE:0621 */

#endif
