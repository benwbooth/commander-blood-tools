/* Codegen probe for BLOODPRG 0x00155F. */

#include <dos.h>

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
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

typedef struct xms_move_request_record_probe {
    u32 length;
    u16 source_handle;
    xms_address_probe source;
    u16 destination_handle;
    xms_address_probe destination;
} xms_move_request_record_probe;

extern volatile i16 GAME_DATA small_xms_handle_probe;
extern volatile i16 GAME_DATA small_ems_handle_probe;
extern volatile u16 GAME_DATA ems_page_frame_segment_probe;
extern volatile u16 GAME_DATA archive_handle_probe;
extern volatile u16 GAME_DATA archive_cache_handle_probe;
extern volatile u8 far *GAME_DATA graphics_work_surface_probe;
extern volatile xms_move_request_record_probe GAME_DATA
        xms_move_request_shared_probe;
extern const volatile char GAME_DATA archive_filename_probe[];
extern const volatile char GAME_DATA archive_cache_filename_probe[];

void far startup_original_directory_restore_probe(void);
void far startup_write_directory_enter_probe(void);
int near dos_open_read_only_probe(
        const volatile char far *path, u16 *handle);
u16 near dos_read_probe(
        u16 handle, volatile u8 far *destination, u16 byte_count);
int near dos_create_game_file_probe(
        const volatile char GAME_DATA *path,
        volatile u16 GAME_DATA *handle);
u16 near dos_write_probe(
        u16 handle, const volatile u8 far *source, u16 byte_count);
void near ems_map_page_probe(
        u16 handle, u16 logical_page, u8 physical_page);
void near xms_move_probe(volatile xms_move_request_record_probe *request);

void near resource_archive_index_backing_initialize_probe(void)
{
    volatile u32 far *source;
    volatile u32 far *destination;
    u16 file_handle;
    u16 work_segment;
    u16 dword_count;
    u8 page;

    startup_original_directory_restore_probe();
    if (!dos_open_read_only_probe(
            (const volatile char far *)archive_filename_probe,
            &file_handle)) {
        return;
    }

    archive_handle_probe = file_handle;
    (void)dos_read_probe(file_handle, graphics_work_surface_probe, 0xffffu);
    work_segment = FP_SEG(graphics_work_surface_probe);

    if (small_ems_handle_probe != -1) {
        page = 0;
        do {
            ems_map_page_probe((u16)small_ems_handle_probe, page, page);
            ++page;
        } while (page != 4u);

        source = (volatile u32 far *)MK_FP(work_segment, 0u);
        destination = (volatile u32 far *)MK_FP(
                ems_page_frame_segment_probe, 0u);
        dword_count = 0x4000u;
        do {
            *destination++ = *source++;
            --dword_count;
        } while (dword_count != 0u);
        return;
    }

    if (small_xms_handle_probe != -1) {
        xms_move_request_shared_probe.length = 0x10000UL;
        xms_move_request_shared_probe.source_handle = 0;
        xms_move_request_shared_probe.source.pointer =
                (volatile u8 far *)MK_FP(work_segment, 0u);
        xms_move_request_shared_probe.destination_handle =
                (u16)small_xms_handle_probe;
        xms_move_request_shared_probe.destination.offset = 0;
        xms_move_probe(&xms_move_request_shared_probe);
        return;
    }

    startup_write_directory_enter_probe();
    (void)dos_create_game_file_probe(
            archive_cache_filename_probe, &archive_cache_handle_probe);
    (void)dos_write_probe(
            archive_cache_handle_probe,
            (const volatile u8 far *)MK_FP(work_segment, 0x00cbu),
            0xffffu);
}
