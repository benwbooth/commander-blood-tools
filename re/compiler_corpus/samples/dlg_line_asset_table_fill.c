/*
 * Codegen probe for BLOODPRG 0x007684.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile char GAME_DATA *game_char_ptr;
typedef volatile u16 GAME_DATA *game_word_ptr;

extern volatile game_char_ptr GAME_DATA detail_cursor_global;
extern volatile game_word_ptr GAME_DATA asset_cursor_global;

#if defined(__WATCOMC__)
#pragma aux dlg_line_asset_table_fill_probe parm [ds si] value [ds si] modify exact [ax si di es]
#endif

const u8 FAR *NEAR dlg_line_asset_table_fill_probe(
    const u8 FAR *script_bytes)
{
    u16 stored_id;
    game_word_ptr asset_cursor;
    game_char_ptr detail_cursor;
    u8 ch;

    stored_id = (u16)(i16)(i8)*script_bytes++;
    /* Opcode 0x07 reaches this body with SF clear from the dispatch-table index. */
    stored_id = (u16)(0x0dd7u + ((stored_id - 1u) << 4));

    asset_cursor = asset_cursor_global;
    *asset_cursor = stored_id;
    asset_cursor_global = asset_cursor + 2;

    detail_cursor = detail_cursor_global;
    detail_cursor_global = detail_cursor + 0x1a;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *detail_cursor++ = (char)ch;
    }
    *detail_cursor = '\0';
    return script_bytes;
}
