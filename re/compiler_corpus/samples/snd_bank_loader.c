/*
 * Codegen probe for BLOODPRG 0x00C005.
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

typedef struct snd_bank_header {
    u16 clip_count;
    u8 dialogue_delay_base;
    u8 dialogue_delay_limit;
} snd_bank_header;

typedef struct snd_memory_clip {
    u16 offset;
    u16 byte_count;
} snd_memory_clip;

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
extern volatile u8 resource_path_is_embedded;
extern volatile i16 secondary_xms_handle;
extern volatile i16 secondary_ems_handle;
extern volatile u16 snd_voice_file_handle;
extern volatile u32 snd_source_remaining;
extern volatile u8 FAR *snd_bank_memory;
extern volatile u8 FAR *graphics_work_surface;
extern volatile u8 FAR ems_page_frame[];
extern volatile snd_bank_header shared_snd_bank_header;
extern volatile snd_memory_clip snd_memory_clips[];
extern volatile u32 snd_source_offsets[];
extern volatile u16 snd_streamed_clip_count;
extern volatile u32 snd_streamed_offsets[];
extern volatile snd_storage_cursor shared_snd_storage_cursor;
extern volatile xms_move_request shared_xms_move_request;
extern const volatile char snd_voice_temp_filename[];

u16 FAR path_builder_probe(volatile char FAR *filename);
u32 FAR resource_name_lookup_probe(volatile char FAR *filename);
int NEAR cb_dos_open_read_only_probe(const volatile char FAR *path, u16 *handle);
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

#if defined(__WATCOMC__)
#pragma aux snd_bank_loader_probe parm [ax] [si] modify exact []
#endif

void FAR snd_bank_loader_probe(u16 mode, volatile char NEAR *path)
{
    volatile u8 FAR *staging;
    u32 payload_remaining;
    u32 clip_start;
    u32 clip_end;
    u16 source_handle;
    u16 table_bytes;
    u16 table_count;
    u16 request_bytes;
    u16 bytes_read;
    u16 logical_page;
    u16 index;

    if ((voc_playback_enabled & 1u) == 0) {
        return;
    }

    source_handle = path_builder_probe(path);
    if ((resource_path_is_embedded & 1u) == 0) {
        snd_source_remaining = resource_name_lookup_probe(path);
        (void)cb_dos_open_read_only_probe(path, &source_handle);
    }

    snd_source_remaining -= 4u;
    (void)cb_dos_read_probe(source_handle,
            (volatile u8 FAR *)&shared_snd_bank_header, 4u);
    table_count = (u16)(shared_snd_bank_header.clip_count + 1u);
    table_bytes = (u16)(table_count * 4u);
    snd_source_remaining -= table_bytes;
    (void)cb_dos_read_probe(source_handle,
            (volatile u8 FAR *)snd_source_offsets, table_bytes);

    if (mode == 0) {
        for (index = 0; index < shared_snd_bank_header.clip_count; ++index) {
            clip_start = snd_source_offsets[index];
            clip_end = snd_source_offsets[index + 1u];
            snd_memory_clips[index].offset = (u16)clip_start;
            snd_memory_clips[index].byte_count =
                    (u16)((u16)(clip_end - clip_start) - 1u);
        }
        (void)cb_dos_read_probe(source_handle, snd_bank_memory,
                (u16)snd_source_remaining);
    } else {
        payload_remaining = snd_source_remaining;
        snd_streamed_clip_count = shared_snd_bank_header.clip_count;
        for (index = 0; index < table_count; ++index) {
            snd_streamed_offsets[index] = snd_source_offsets[index];
        }

        if (secondary_ems_handle != -1) {
            shared_snd_storage_cursor.ems.logical_page = 0;
            while (payload_remaining != 0) {
                logical_page = shared_snd_storage_cursor.ems.logical_page;
                cb_ems_map_page_probe((u16)secondary_ems_handle,
                        logical_page, 0);
                ++logical_page;
                cb_ems_map_page_probe((u16)secondary_ems_handle,
                        logical_page, 1u);
                ++logical_page;
                shared_snd_storage_cursor.ems.logical_page = logical_page;

                request_bytes = payload_remaining > 0x8000UL
                        ? 0x8000u
                        : (u16)payload_remaining;
                bytes_read = cb_dos_read_probe(source_handle,
                        ems_page_frame, request_bytes);
                payload_remaining -= bytes_read;
            }
        } else {
            staging = graphics_work_surface + 0x7d00u;
            if (secondary_xms_handle != -1) {
                shared_snd_storage_cursor.xms_offset = 0;
                while (payload_remaining != 0) {
                    request_bytes = payload_remaining > 0x7d00UL
                            ? 0x7d00u
                            : (u16)payload_remaining;
                    bytes_read = cb_dos_read_probe(source_handle,
                            staging, request_bytes);

                    shared_xms_move_request.length =
                            (u32)bytes_read + (bytes_read & 1u);
                    shared_xms_move_request.source_handle = 0;
                    shared_xms_move_request.source.pointer = staging;
                    shared_xms_move_request.destination_handle =
                            (u16)secondary_xms_handle;
                    shared_xms_move_request.destination.offset =
                            shared_snd_storage_cursor.xms_offset;
                    shared_snd_storage_cursor.xms_offset += 0x7d00UL;
                    cb_xms_move_probe(&shared_xms_move_request);
                    payload_remaining -= bytes_read;
                }
            } else {
                if (snd_voice_file_handle != 0) {
                    cb_dos_close_probe(snd_voice_file_handle);
                }
                startup_write_directory_enter_probe();
                (void)cb_dos_create_game_file_probe(
                        snd_voice_temp_filename, &snd_voice_file_handle);

                while (payload_remaining != 0) {
                    request_bytes = payload_remaining > 0x7d00UL
                            ? 0x7d00u
                            : (u16)payload_remaining;
                    bytes_read = cb_dos_read_probe(source_handle,
                            staging, request_bytes);
                    (void)cb_dos_write_probe(snd_voice_file_handle,
                            staging, bytes_read);
                    payload_remaining -= bytes_read;
                }
            }
        }
    }

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close_probe(source_handle);
    }
}
