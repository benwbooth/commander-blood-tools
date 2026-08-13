/*
 * Codegen probe for BLOODPRG 0x0026CF.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA far
#endif

typedef union xms_address_probe {
    u32 offset;
    volatile u8 far *pointer;
} xms_address_probe;

typedef struct xms_move_request_probe {
    u32 length;
    u16 source_handle;
    xms_address_probe source;
    u16 destination_handle;
    xms_address_probe destination;
} xms_move_request_probe;

#pragma pack(1)
typedef struct resource_archive_entry_probe {
    char filename[16];
    u32 byte_count;
    u32 file_offset;
    u8 unknown_18;
} resource_archive_entry_probe;
#pragma pack()

extern volatile u16 GAME_DATA archive_handle_probe;
extern volatile u16 GAME_DATA archive_cache_handle_probe;
extern volatile u32 GAME_DATA archive_offset_probe;
extern volatile u32 GAME_DATA archive_remaining_probe;
extern volatile u32 GAME_DATA source_remaining_probe;
extern volatile u8 GAME_DATA path_is_embedded_probe;
extern volatile i16 GAME_DATA small_xms_handle_probe;
extern volatile i16 GAME_DATA small_ems_handle_probe;
extern volatile u16 GAME_DATA ems_page_frame_segment_probe;
extern volatile u8 far *GAME_DATA graphics_work_surface_probe;
extern volatile xms_move_request_probe GAME_DATA xms_move_request_shared_probe;

void near ems_map_page_probe(u16 handle, u16 logical_page, u8 physical_page);
void near xms_move_probe(volatile xms_move_request_probe *request);
u16 near dos_read_probe(u16 handle, volatile u8 far *destination, u16 byte_count);
void near dos_seek_absolute_probe(u16 handle, u32 offset);

u16 near resource_archive_match_probe(volatile char far *filename)
{
    volatile resource_archive_entry_probe far *entry;
    volatile u8 far *archive_index;
    volatile u8 far *character;
    u32 payload_offset;
    u16 archive_handle;
    u16 character_index;
    u8 archive_character;
    u8 filename_character;
    u8 physical_page;

    archive_handle = archive_handle_probe;
    if (archive_handle == 0) {
        return 0;
    }

    if (small_ems_handle_probe != -1) {
        for (physical_page = 0; physical_page < 4u; ++physical_page) {
            ems_map_page_probe((u16)small_ems_handle_probe,
                    physical_page, physical_page);
        }
        archive_index = (volatile u8 far *)MK_FP(
                ems_page_frame_segment_probe, 0u);
    } else {
        archive_index = graphics_work_surface_probe + 0x7d00u;
        if (small_xms_handle_probe != -1) {
            xms_move_request_shared_probe.length = 0x7d00UL;
            xms_move_request_shared_probe.source_handle =
                    (u16)small_xms_handle_probe;
            xms_move_request_shared_probe.source.offset = 0;
            xms_move_request_shared_probe.destination_handle = 0;
            xms_move_request_shared_probe.destination.pointer = archive_index;
            xms_move_probe(&xms_move_request_shared_probe);
        } else {
            (void)dos_read_probe(archive_cache_handle_probe,
                    archive_index, 0xffffu);
            archive_index = (volatile u8 far *)MK_FP(
                    FP_SEG(graphics_work_surface_probe), 0x7d00u);
        }
    }

    character = (volatile u8 far *)filename;
    do {
        filename_character = *character;
        if (filename_character >= (u8)'a') {
            filename_character &= 0xdfu;
            *character = filename_character;
        }
        ++character;
    } while (filename_character != 0);

    entry = (volatile resource_archive_entry_probe far *)(archive_index + 2u);
    while (entry->filename[0] != '\0') {
        character_index = 0;
        do {
            archive_character =
                    ((volatile u8 far *)entry->filename)[character_index];
            filename_character =
                    ((volatile u8 far *)filename)[character_index];
            if (archive_character != filename_character) {
                break;
            }
            ++character_index;
        } while (archive_character != 0);

        if (archive_character == 0 && filename_character == 0) {
            path_is_embedded_probe = 1;
            archive_remaining_probe = entry->byte_count;
            source_remaining_probe = entry->byte_count;
            payload_offset = entry->file_offset;
            archive_offset_probe = payload_offset;
            dos_seek_absolute_probe(archive_handle, payload_offset);
            return archive_handle;
        }
        ++entry;
    }

    return 0;
}
