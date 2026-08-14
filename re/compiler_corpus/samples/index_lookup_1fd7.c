/*
 * Codegen probe for BLOODPRG 0x0076EA.
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
typedef volatile u8 FAR *far_u8_ptr;

extern volatile u16 GAME_DATA index_word;
extern volatile char GAME_DATA index_path[];
extern volatile char GAME_DATA index_text[];
extern volatile u16 GAME_DATA ui_state;
extern volatile i16 GAME_DATA xms_handle;
extern volatile i16 GAME_DATA ems_handle;
extern volatile far_u8_ptr GAME_DATA back_buffer;

void FAR path_build_probe(game_char_ptr path);
void FAR file_open_probe(game_char_ptr path, far_u8_ptr destination);

#if defined(__WATCOMC__)
#pragma aux index_lookup_1fd7_probe parm [ds si] value [ds si] modify exact [ax si es]
#endif

const u8 FAR *NEAR index_lookup_1fd7_probe(const u8 FAR *script_bytes)
{
    u16 stored_id;
    game_char_ptr dst;
    u8 ch;

    stored_id = (u16)(i16)(i8)*script_bytes++;
    /* Opcode 0x0B reaches this body with SF clear from the dispatch-table index. */
    stored_id = (u16)(0x0dd7u + ((stored_id - 1u) << 4));
    index_word = stored_id;

    dst = index_text;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';

    if ((ui_state & 1u) == 0) {
        if (ems_handle != -1) {
            path_build_probe(index_path);
        } else if (xms_handle != -1) {
            file_open_probe(index_path, back_buffer);
        }
    }
    return script_bytes;
}
