/* Codegen probe for BLOODPRG 0x007409. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;
typedef signed char i8;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))

typedef volatile char GAME_DATA *game_char_ptr;
typedef volatile u16 GAME_DATA *game_word_ptr;
typedef volatile u8 FAR *far_u8_ptr;

extern volatile u8 GAME_DATA fs_name_area_dirty_probe;
extern volatile u8 GAME_DATA music_voc_name_changed_probe;
extern volatile u8 GAME_DATA byte_parser_table_count_probe;
extern volatile game_char_ptr GAME_DATA byte_parser_detail_cursor_probe;
extern volatile game_char_ptr GAME_DATA byte_parser_table_cursor_probe;
extern volatile game_char_ptr GAME_DATA byte_parser_stream_cursor_probe;
extern volatile game_word_ptr GAME_DATA byte_parser_asset_cursor_probe;
extern volatile u8 FAR byte_parser_stop_flag_probe;
extern volatile char GAME_DATA vm_record_string_slots_probe[][16];
extern volatile char GAME_DATA descript_database_path_probe[];
extern volatile u16 GAME_DATA descript_directory_count_probe;
extern volatile u16 GAME_DATA descript_record_length_probe;
extern far_u8_ptr GAME_DATA graphics_work_surface_probe;

extern u16 FAR resource_source_select_probe(volatile char FAR *path);
extern int NEAR dos_open_probe(const volatile char FAR *path, u16 *handle);
extern u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
extern void NEAR dos_seek_probe(u16 handle, u32 offset);
extern void NEAR dos_close_probe(u16 handle);

extern void NEAR op_01_probe(void);
extern void NEAR op_02_probe(void);
extern void NEAR op_04_probe(void);
extern void NEAR op_0f_probe(void);
extern const u8 FAR *NEAR op_03_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_05_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_06_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_07_probe(const u8 FAR *script);
extern const u16 FAR *NEAR op_08_probe(const u16 FAR *script);
extern const u8 FAR *NEAR op_09_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_0a_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_0b_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_0c_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_0d_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_0e_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_10_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_11_probe(const u8 FAR *script);
extern const u8 FAR *NEAR op_12_probe(const u8 FAR *script);

#pragma aux op_01_probe modify exact []
#pragma aux op_02_probe modify exact []
#pragma aux op_04_probe modify exact []
#pragma aux op_0f_probe modify exact []
#pragma aux op_03_probe parm [ds si] value [ds si] \
        modify exact [ax bx cx dx si di bp es]
#pragma aux op_05_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_06_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_07_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_08_probe parm [ds si] value [ds si] modify exact [ax si es]
#pragma aux op_09_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_0a_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_0b_probe parm [ds si] value [ds si] modify exact [ax si es]
#pragma aux op_0c_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_0d_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_0e_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_10_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux op_11_probe parm [ds si] value [ds si] \
        modify exact [ax bx cx dx si di es]
#pragma aux op_12_probe parm [ds si] value [ds si] modify exact [ax si di es]
#pragma aux vm_c2_descript_lookup_probe parm [es di] value [ax] modify exact [ax]

#pragma pack(1)
typedef struct descript_directory_entry_probe {
    u8 name[16];
    u16 record_offset;
} descript_directory_entry_probe;
#pragma pack()

int FAR vm_c2_descript_lookup_probe(const volatile u8 FAR *record_name)
{
    volatile descript_directory_entry_probe FAR *directory;
    const volatile u8 FAR *directory_name;
    const volatile u8 FAR *wanted_name;
    const u8 FAR *script;
    u16 file_handle;
    u16 directory_bytes;
    u8 directory_character;
    u8 wanted_character;
    u8 opcode;
    int found;
    int result;

    fs_name_area_dirty_probe = 0;
    music_voc_name_changed_probe = 0;
    byte_parser_table_count_probe = 0;
    byte_parser_detail_cursor_probe = (game_char_ptr)0x2154u;
    byte_parser_table_cursor_probe = (game_char_ptr)(
            0x1320u + ((u16)(u8)vm_record_string_slots_probe[0][0] << 7));
    byte_parser_stream_cursor_probe = (game_char_ptr)0x0f1au;
    byte_parser_asset_cursor_probe = (game_word_ptr)0x1fdbu;
    byte_parser_stop_flag_probe = 0;

    (void)resource_source_select_probe(descript_database_path_probe);
    if (!dos_open_probe(descript_database_path_probe, &file_handle)) {
        return 0;
    }
    (void)dos_read_probe(file_handle,
            (volatile u8 FAR *)&descript_directory_count_probe, 2u);
    directory_bytes = (u16)(descript_directory_count_probe * 18u);
    (void)dos_read_probe(file_handle, graphics_work_surface_probe,
            directory_bytes);

    directory = (volatile descript_directory_entry_probe FAR *)
            graphics_work_surface_probe;
    found = 0;
    for (;;) {
        directory_name = directory->name;
        wanted_name = record_name;
        for (;;) {
            directory_character = *directory_name++;
            wanted_character = *wanted_name++;
            if (directory_character == 0 && wanted_character == 0) {
                found = 1;
                break;
            }
            if (directory_character != wanted_character) {
                break;
            }
        }
        if (found) {
            break;
        }
        ++directory;
        --descript_directory_count_probe;
        if (descript_directory_count_probe == 0) {
            break;
        }
    }

    result = 0;
    if (found) {
        dos_seek_probe(file_handle, (u32)directory->record_offset);
        (void)dos_read_probe(file_handle,
                (volatile u8 FAR *)&descript_record_length_probe, 2u);
        (void)dos_read_probe(file_handle, graphics_work_surface_probe,
                (u16)(descript_record_length_probe - 2u));
        script = (const u8 FAR *)graphics_work_surface_probe;
        for (;;) {
            opcode = *script++;
            if ((i8)(opcode - 1u) < 0) {
                break;
            }
            switch (opcode) {
            case 0x01: op_01_probe(); break;
            case 0x02: op_02_probe(); break;
            case 0x03: script = op_03_probe(script); break;
            case 0x04: op_04_probe(); break;
            case 0x05: script = op_05_probe(script); break;
            case 0x06: script = op_06_probe(script); break;
            case 0x07: script = op_07_probe(script); break;
            case 0x08:
                script = (const u8 FAR *)op_08_probe((const u16 FAR *)script);
                break;
            case 0x09: script = op_09_probe(script); break;
            case 0x0a: script = op_0a_probe(script); break;
            case 0x0b: script = op_0b_probe(script); break;
            case 0x0c: script = op_0c_probe(script); break;
            case 0x0d: script = op_0d_probe(script); break;
            case 0x0e: script = op_0e_probe(script); break;
            case 0x0f: op_0f_probe(); break;
            case 0x10: script = op_10_probe(script); break;
            case 0x11: script = op_11_probe(script); break;
            case 0x12: script = op_12_probe(script); break;
            }
            if ((byte_parser_stop_flag_probe & 1u) != 0) {
                break;
            }
        }
        result = 1;
    }

    *(game_word_ptr)byte_parser_stream_cursor_probe = 0xffffu;
    byte_parser_stream_cursor_probe = (game_char_ptr)0x0f1au;
    dos_close_probe(file_handle);
    return result;
}
