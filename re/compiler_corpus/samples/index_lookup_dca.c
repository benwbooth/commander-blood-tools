/* Codegen probe for BLOODPRG 0x00755E. */
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#define BACKGROUND_NAME_OFFSET 3u

typedef volatile u8 FAR *far_u8_ptr;

extern volatile char GAME_DATA background_path_probe[];
extern volatile char GAME_DATA background_slots_probe[][16];
extern volatile u32 GAME_DATA background_source_size_probe;
extern volatile u8 GAME_DATA source_is_embedded_probe;
extern far_u8_ptr GAME_DATA back_buffer_probe;

extern void FAR write_directory_enter_probe(void);
extern int NEAR dos_delete_probe(const volatile char FAR *path);
extern int NEAR dos_create_probe(
        const volatile char FAR *path, u16 *handle);
extern u16 FAR source_select_probe(volatile char FAR *path);
extern u32 FAR resource_lookup_probe(volatile char FAR *path);
extern int NEAR dos_open_probe(const volatile char FAR *path, u16 *handle);
extern u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
extern u16 NEAR dos_write_probe(
        u16 handle, const volatile u8 FAR *source, u16 byte_count);
extern void NEAR dos_close_probe(u16 handle);

#pragma aux index_lookup_dca_probe parm [ds si] value [ds si] \
        modify exact [ax bx cx dx si di bp es]

const u8 FAR *NEAR index_lookup_dca_probe(const u8 FAR *script)
{
    u16 slot;
    u16 name_index;
    u16 compare_index;
    u16 source_handle;
    u16 output_handle;
    u16 bytes_read;
    u8 value;
    int names_match;

    slot = (u16)(i16)(i8)(u8)(*script++ - 1u);
    name_index = 0;
    for (;;) {
        value = *script;
        if ((i8)value < 0 || value < 0x20u) {
            break;
        }
        ++script;
        background_path_probe[
                BACKGROUND_NAME_OFFSET + name_index++] = (char)value;
    }
    background_path_probe[BACKGROUND_NAME_OFFSET + name_index] = '\0';

    compare_index = 0;
    names_match = 1;
    while (background_path_probe[
            BACKGROUND_NAME_OFFSET + compare_index] != '\0') {
        if (background_path_probe[BACKGROUND_NAME_OFFSET + compare_index]
                != background_slots_probe[slot][compare_index]) {
            names_match = 0;
            break;
        }
        ++compare_index;
    }
    if (names_match) {
        return script;
    }

    write_directory_enter_probe();
    (void)dos_delete_probe(background_slots_probe[slot]);
    name_index = 0;
    do {
        value = (u8)background_path_probe[
                BACKGROUND_NAME_OFFSET + name_index];
        background_slots_probe[slot][name_index++] = (char)value;
    } while (value != 0);

    (void)dos_create_probe(background_slots_probe[slot], &output_handle);
    source_handle = source_select_probe(background_path_probe);
    if ((source_is_embedded_probe & 1u) == 0) {
        background_source_size_probe =
                resource_lookup_probe(background_path_probe);
        (void)dos_open_probe(background_path_probe, &source_handle);
    }
    bytes_read = dos_read_probe(source_handle, back_buffer_probe,
            (u16)background_source_size_probe);
    (void)dos_write_probe(output_handle, back_buffer_probe, bytes_read);
    if ((source_is_embedded_probe & 1u) == 0) {
        dos_close_probe(source_handle);
    }
    dos_close_probe(output_handle);
    return script;
}
