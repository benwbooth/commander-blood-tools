#ifndef BLOODPRG_BYTE_PARSER_H
#define BLOODPRG_BYTE_PARSER_H

#include "bloodprg_common.h"
#include "bloodprg_ems.h"

#if defined(__WATCOMC__)
#define CB_GAME_DATA __based(__segname("GAME_DATA"))
#else
#define CB_GAME_DATA CB_FAR
#endif

extern volatile cb_u8 byte_parser_b16_flag;  /* GS:0x0B16 */
extern volatile char byte_parser_table_2460[]; /* GS:0x2460 */
extern volatile char byte_parser_table_247a[]; /* GS:0x247A */
extern volatile char byte_parser_line_name[]; /* GS:0x24C6 */
extern volatile char byte_parser_text_20b8[]; /* GS:0x20B8 */
extern volatile char fs_resource_name_area[]; /* FS:0x0C74 */
extern char CB_GAME_DATA credit_text_buffer[];          /* ES:0x0E18 */
extern volatile cb_u8 CB_GAME_DATA credit_reveal_active; /* GS:0x5E64 */
extern volatile cb_u16 CB_GAME_DATA credit_reveal_timer; /* GS:0x5E58 */
extern volatile cb_u8 fs_name_area_dirty;    /* GS:0x27E8 */
extern volatile char music_voc_name_field[]; /* GS:0x0D30 */
extern volatile cb_u8 music_voc_name_unchanged; /* GS:0x0BA0 */
extern volatile cb_u8 music_voc_name_changed; /* GS:0x0BA1 */
extern volatile char byte_parser_snd_bank_path[]; /* GS:0x0D06 */
extern volatile char byte_parser_snd_bank_name_field[]; /* GS:0x0D09 */
extern volatile cb_u16 byte_parser_index_word_1fd7; /* GS:0x1FD7 */
extern volatile char byte_parser_index_path_2137[]; /* GS:0x2137 */
extern volatile char byte_parser_index_text_213a[]; /* GS:0x213A */
extern volatile cb_u8 CB_FAR *byte_parser_back_buffer; /* GS:0x5229 */
extern volatile cb_u16 byte_parser_word_1fa5; /* GS:0x1FA5 */
extern volatile char *byte_parser_detail_cursor; /* GS:0x1FAD */
extern volatile cb_u16 *byte_parser_asset_cursor; /* GS:0x1FAF */
extern volatile char *byte_parser_table_131a_cursor; /* GS:0x131A */
extern volatile cb_u8 byte_parser_table_131e_count; /* GS:0x131E */
extern volatile char *byte_parser_stream_0f18_cursor; /* GS:0x0F18 */

#if defined(__WATCOMC__)
#pragma aux byte_parser_op_01_mark_b16 modify exact []
#pragma aux byte_parser_op_02_mark_b16 modify exact []
#pragma aux byte_parser_op_0f_mark_b16 modify exact []
#pragma aux byte_parser_op_04_mark_b16 modify exact []
#pragma aux credit_presenter_b_cryo parm [si] value [si] modify exact [ax si di]
#endif

void CB_NEAR byte_parser_op_01_mark_b16(void); /* 0x007542 */
void CB_NEAR byte_parser_op_02_mark_b16(void); /* 0x007549 */
void CB_NEAR byte_parser_op_0f_mark_b16(void); /* 0x007550 */
void CB_NEAR byte_parser_op_04_mark_b16(void); /* 0x007557 */
const cb_u8 CB_NEAR *CB_NEAR credit_presenter_b_cryo(
    const cb_u8 CB_NEAR *script_bytes); /* 0x007612 */

void CB_FAR path_build_call_2693(const volatile char *path); /* 0x01CE:0712 */
void CB_FAR file_open_wrapper(const volatile char *path,
        volatile cb_u8 CB_FAR *destination); /* 0x01CE:0621 */

#endif
