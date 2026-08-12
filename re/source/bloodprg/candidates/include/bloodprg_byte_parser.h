#ifndef BLOODPRG_BYTE_PARSER_H
#define BLOODPRG_BYTE_PARSER_H

#include "bloodprg_common.h"
#include "bloodprg_ems.h"

#if defined(__WATCOMC__)
#define CB_GAME_DATA __based(__segname("GAME_DATA"))
#else
#define CB_GAME_DATA CB_FAR
#endif

typedef volatile char CB_GAME_DATA *cb_game_char_ptr;
typedef volatile cb_u16 CB_GAME_DATA *cb_game_word_ptr;

extern volatile cb_u8 byte_parser_b16_flag;  /* GS:0x0B16 */
extern char CB_GAME_DATA byte_parser_table_2460[]; /* ES:0x2460 */
extern char CB_GAME_DATA byte_parser_table_247a[]; /* ES:0x247A */
extern char CB_GAME_DATA byte_parser_line_name[];  /* ES:0x24C6 */
extern char CB_GAME_DATA byte_parser_text_20b8[];  /* ES:0x20B8 */
extern volatile char fs_resource_name_area[]; /* FS:0x0C74 */
extern char CB_GAME_DATA credit_text_buffer[];          /* ES:0x0E18 */
extern volatile cb_u8 CB_GAME_DATA credit_reveal_active; /* GS:0x5E64 */
extern volatile cb_u16 CB_GAME_DATA credit_reveal_timer; /* GS:0x5E58 */
extern volatile cb_u8 fs_name_area_dirty;    /* GS:0x27E8 */
extern volatile char music_voc_name_field[]; /* GS:0x0D30 */
extern volatile cb_u8 music_voc_name_unchanged; /* GS:0x0BA0 */
extern volatile cb_u8 music_voc_name_changed; /* GS:0x0BA1 */
extern volatile char CB_GAME_DATA byte_parser_snd_bank_path[]; /* GS:0x0D06 */
extern char CB_GAME_DATA byte_parser_snd_bank_name_field[]; /* ES:0x0D09 */
extern volatile cb_u16 CB_GAME_DATA byte_parser_ui_state; /* GS:0x2793 */
extern volatile cb_u16 byte_parser_index_word_1fd7; /* GS:0x1FD7 */
extern volatile char byte_parser_index_path_2137[]; /* GS:0x2137 */
extern volatile char byte_parser_index_text_213a[]; /* GS:0x213A */
extern volatile cb_u8 CB_FAR *byte_parser_back_buffer; /* GS:0x5229 */
extern volatile cb_u16 CB_GAME_DATA byte_parser_word_1fa5; /* GS:0x1FA5 */
extern volatile cb_game_char_ptr CB_GAME_DATA byte_parser_detail_cursor; /* GS:0x1FAD */
extern volatile cb_game_word_ptr CB_GAME_DATA byte_parser_asset_cursor; /* GS:0x1FAF */
extern volatile char *byte_parser_table_131a_cursor; /* GS:0x131A */
extern volatile cb_u8 byte_parser_table_131e_count; /* GS:0x131E */
extern volatile char *byte_parser_stream_0f18_cursor; /* GS:0x0F18 */

#if defined(__WATCOMC__)
#pragma aux byte_parser_op_01_mark_b16 modify exact []
#pragma aux byte_parser_op_02_mark_b16 modify exact []
#pragma aux byte_parser_op_0f_mark_b16 modify exact []
#pragma aux byte_parser_op_04_mark_b16 modify exact []
#pragma aux credit_presenter_b_cryo parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_copy_20b8_printable parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_copy_24c6_printable parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_copy_2460_printable parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_copy_247a_printable parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_snd_bank_name_load parm [si] value [si] modify exact [ax bx cx dx si di es]
#pragma aux dlg_line_asset_table_fill parm [si] value [si] modify exact [ax si di]
#pragma aux byte_parser_store_word_1fa5 parm [si] value [si] modify exact [ax si]
#endif

void CB_NEAR byte_parser_op_01_mark_b16(void); /* 0x007542 */
void CB_NEAR byte_parser_op_02_mark_b16(void); /* 0x007549 */
void CB_NEAR byte_parser_op_0f_mark_b16(void); /* 0x007550 */
void CB_NEAR byte_parser_op_04_mark_b16(void); /* 0x007557 */
const cb_u8 CB_NEAR *CB_NEAR credit_presenter_b_cryo(
    const cb_u8 CB_NEAR *script_bytes); /* 0x007612 */
const cb_u8 CB_NEAR *CB_NEAR byte_parser_copy_20b8_printable(
    const cb_u8 CB_NEAR *script_bytes); /* 0x007629 */
const cb_u8 CB_NEAR *CB_NEAR byte_parser_copy_24c6_printable(
    const cb_u8 CB_NEAR *script_bytes); /* 0x00766F */
const cb_u8 CB_NEAR *CB_NEAR byte_parser_copy_2460_printable(
    const cb_u8 CB_NEAR *script_bytes); /* 0x0076C0 */
const cb_u8 CB_NEAR *CB_NEAR byte_parser_copy_247a_printable(
    const cb_u8 CB_NEAR *script_bytes); /* 0x0076D5 */
const cb_u8 CB_NEAR *CB_NEAR byte_parser_snd_bank_name_load(
    const cb_u8 CB_NEAR *script_bytes); /* 0x00763E */
const cb_u8 CB_NEAR *CB_NEAR dlg_line_asset_table_fill(
    const cb_u8 CB_NEAR *script_bytes); /* 0x007684 */
const cb_u16 CB_NEAR *CB_NEAR byte_parser_store_word_1fa5(
    const cb_u16 CB_NEAR *script_words); /* 0x0076BA */

void CB_FAR path_build_call_2693(const volatile char *path); /* 0x01CE:0712 */
void CB_FAR file_open_wrapper(const volatile char *path,
        volatile cb_u8 CB_FAR *destination); /* 0x01CE:0621 */

#endif
