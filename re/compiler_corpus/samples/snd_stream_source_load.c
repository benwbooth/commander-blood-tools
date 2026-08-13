/*
 * Codegen probe for BLOODPRG 0x00BDB7.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef volatile u8 FAR *graphics_buffer_ptr;

typedef union xms_address {
    u32 offset;
    volatile u8 FAR *pointer;
} xms_address;

typedef struct xms_move_request {
    u32 length;
    u16 source_handle;
    xms_address source;
    u16 destination_handle;
    xms_address destination;
} xms_move_request;

typedef union snd_storage_cursor {
    u32 xms_offset;
    struct {
        u16 logical_page;
        u16 reserved;
    } ems;
} snd_storage_cursor;

extern volatile u8 voc_playback_enabled;
extern volatile u8 snd_stream_channel_active;
extern volatile u8 resource_path_is_embedded;
extern volatile u8 music_voc_name_changed;
extern volatile u8 snd_driver_pending_flag;
extern volatile u8 snd_bank_storage_mode;
extern volatile u8 vm_subtitle_display_mode;
extern volatile u8 vm_presentation_hold_ready;
extern volatile u8 vm_presentation_defer_a;
extern volatile i16 snd_bank_ems_handle;
extern volatile i16 snd_bank_xms_handle;
extern volatile u16 snd_bank_file_handle;
extern volatile u16 snd_stream_page_count;
extern volatile u16 snd_stream_final_page_bytes;
extern volatile u16 vm_text_reveal_cursor;
extern volatile u16 vm_text_reveal_phase;
extern volatile u32 resource_archive_offset;
extern volatile u32 snd_stream_source_remaining;
extern volatile char vm_text_buffer[];
extern const volatile char snd_wait_prompt_text[];
extern const volatile char snd_music_temp_filename[];
extern volatile u8 FAR ems_page_frame[];
extern volatile u8 FAR *snd_stream_storage;
extern volatile xms_move_request shared_xms_move_request;
extern volatile snd_storage_cursor shared_snd_storage_cursor;
extern graphics_buffer_ptr graphics_draw_framebuffer;
extern graphics_buffer_ptr graphics_screen_buffer;

u16 FAR path_builder_probe(const volatile char *filename);
u32 FAR resource_name_lookup_probe(const volatile char *filename);
int NEAR cb_dos_open_read_only_probe(const volatile char *path, u16 *handle);
void NEAR cb_dos_seek_absolute_probe(u16 handle, u32 offset);
u16 NEAR cb_dos_read_probe(u16 handle,
        volatile u8 FAR *destination, u16 byte_count);
void NEAR cb_dos_close_probe(u16 handle);
int NEAR cb_dos_create_game_file_probe(
        const volatile char *path, volatile u16 *handle);
u16 NEAR cb_dos_write_probe(u16 handle,
        const volatile u8 FAR *source, u16 byte_count);
void NEAR cb_ems_map_page_probe(u16 handle, u16 logical_page,
        u8 physical_page);
void NEAR cb_xms_move_probe(volatile xms_move_request *request);
void FAR startup_write_directory_enter_probe(void);
void FAR subtitle_reveal_pump_probe(void);

#if defined(__WATCOMC__)
#pragma aux snd_stream_source_load_probe parm [si] modify exact []
#pragma aux path_builder_probe parm [dx] value [bx] modify [bx cx dx]
#endif

void FAR snd_stream_source_load_probe(const volatile char NEAR *path)
{
    graphics_buffer_ptr saved_framebuffer;
    u32 seek_offset;
    u16 source_handle;
    u16 request_bytes;
    u16 bytes_read;
    u16 logical_page;
    u16 remainder;
    u16 index;
    u8 character;

    if ((voc_playback_enabled & 1u) == 0
            || (snd_stream_channel_active & 1u) == 0) {
        return;
    }

    source_handle = path_builder_probe(path);
    if ((resource_path_is_embedded & 1u) == 0) {
        snd_stream_source_remaining = resource_name_lookup_probe(path);
        resource_archive_offset = 0;
        (void)cb_dos_open_read_only_probe(path, &source_handle);
    }

    snd_stream_page_count = 0;
    music_voc_name_changed = 0;
    snd_driver_pending_flag = 1u;

    vm_text_reveal_cursor = 0x0e2au;
    index = 0;
    do {
        character = (u8)snd_wait_prompt_text[index];
        vm_text_buffer[index] = (char)character;
        ++index;
    } while (character != 0);
    vm_subtitle_display_mode = 2u;
    vm_text_reveal_phase = 0;
    vm_presentation_hold_ready = 0;
    saved_framebuffer = graphics_draw_framebuffer;
    graphics_draw_framebuffer = graphics_screen_buffer;
    subtitle_reveal_pump_probe();
    graphics_draw_framebuffer = saved_framebuffer;
    vm_subtitle_display_mode = 0;
    vm_presentation_defer_a = 0;

    seek_offset = resource_archive_offset;
    seek_offset = (seek_offset & 0xffff0000UL)
            | (u16)((u16)seek_offset + 0x001au);
    cb_dos_seek_absolute_probe(source_handle, seek_offset);
    snd_stream_source_remaining -= 0x1aUL;

    bytes_read = 0;
    if (snd_bank_ems_handle != -1) {
        snd_bank_storage_mode = 0;
        shared_snd_storage_cursor.ems.logical_page = 0;
        while (snd_stream_source_remaining != 0) {
            logical_page = shared_snd_storage_cursor.ems.logical_page;
            cb_ems_map_page_probe((u16)snd_bank_ems_handle,
                    logical_page, 0);
            ++logical_page;
            cb_ems_map_page_probe((u16)snd_bank_ems_handle,
                    logical_page, 1u);
            ++logical_page;
            shared_snd_storage_cursor.ems.logical_page = logical_page;

            request_bytes = snd_stream_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (u16)snd_stream_source_remaining;
            bytes_read = cb_dos_read_probe(source_handle,
                    ems_page_frame, request_bytes);
            snd_stream_page_count += 2u;
            snd_stream_source_remaining -= bytes_read;
        }
    } else if (snd_bank_xms_handle != -1) {
        snd_bank_storage_mode = 1u;
        shared_snd_storage_cursor.xms_offset = 0;
        while (snd_stream_source_remaining != 0) {
            request_bytes = snd_stream_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (u16)snd_stream_source_remaining;
            bytes_read = cb_dos_read_probe(source_handle,
                    snd_stream_storage, request_bytes);
            if (bytes_read == 0) {
                break;
            }

            shared_xms_move_request.length =
                    (u32)bytes_read + (bytes_read & 1u);
            shared_xms_move_request.source_handle = 0;
            shared_xms_move_request.source.pointer = snd_stream_storage;
            shared_xms_move_request.destination_handle =
                    (u16)snd_bank_xms_handle;
            shared_xms_move_request.destination.offset =
                    shared_snd_storage_cursor.xms_offset;
            shared_snd_storage_cursor.xms_offset += 0x8000UL;
            cb_xms_move_probe(&shared_xms_move_request);

            snd_stream_page_count += 2u;
            snd_stream_source_remaining -= bytes_read;
        }
    } else {
        snd_bank_storage_mode = 2u;
        if (snd_bank_file_handle != 0) {
            cb_dos_close_probe(snd_bank_file_handle);
        }
        startup_write_directory_enter_probe();
        (void)cb_dos_create_game_file_probe(
                snd_music_temp_filename, &snd_bank_file_handle);

        while (snd_stream_source_remaining != 0) {
            request_bytes = snd_stream_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (u16)snd_stream_source_remaining;
            bytes_read = cb_dos_read_probe(source_handle,
                    snd_stream_storage, request_bytes);
            (void)cb_dos_write_probe(snd_bank_file_handle,
                    snd_stream_storage, bytes_read);
            snd_stream_page_count += 2u;
            snd_stream_source_remaining -= bytes_read;
        }
    }

    remainder = bytes_read & 0x3fffu;
    if (remainder == bytes_read) {
        --snd_stream_page_count;
    }
    snd_stream_final_page_bytes = remainder != 0 ? remainder : 0x4000u;

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close_probe(source_handle);
    }
}
