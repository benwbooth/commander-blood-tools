/*
 * Codegen probe for BLOODPRG 0x002901.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef signed long i32;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define XMS_READ_BYTES 0x7d00UL

typedef struct dos_dta_probe {
    u8 reserved_00[0x1a];
    u32 file_size;
} dos_dta_probe;

typedef union xms_address_probe {
    u32 offset;
    volatile u8 FAR *pointer;
} xms_address_probe;

typedef struct xms_move_request_probe {
    u32 length;
    u16 source_handle;
    xms_address_probe source;
    u16 destination_handle;
    xms_address_probe destination;
} xms_move_request_probe;

extern volatile u32 archive_remaining_probe;
extern volatile u32 source_remaining_probe;
extern volatile u32 archive_size_probe;
extern volatile u32 storage_cursor_probe;
extern volatile u8 path_is_embedded_probe;
extern volatile i16 xms_handle_probe;
extern volatile u16 file_handle_probe;
extern volatile xms_move_request_probe xms_move_request_shared_probe;

u16 FAR resource_source_select_probe(volatile char FAR *path);
volatile dos_dta_probe FAR *NEAR dos_get_dta_probe(void);
int NEAR dos_find_first_probe(const volatile char FAR *path);
int NEAR dos_open_read_only_probe(
        const volatile char FAR *path, u16 *handle);
u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
void NEAR xms_move_probe(volatile xms_move_request_probe *request);
void NEAR dos_close_probe(u16 handle);

void FAR resource_file_load_to_xms_probe(volatile char FAR *path,
        volatile u8 FAR *staging_buffer)
{
    volatile dos_dta_probe FAR *dta;
    u32 difference;
    u32 file_size;
    u16 file_handle;
    u16 request_bytes;
    u16 bytes_read;
    u16 move_bytes;

    file_handle = resource_source_select_probe(path);
    if ((path_is_embedded_probe & 1u) == 0) {
        dta = dos_get_dta_probe();
        (void)dos_find_first_probe(path);
        file_size = dta->file_size;
        archive_remaining_probe = file_size;
        source_remaining_probe = file_size;
        if (!dos_open_read_only_probe(path, &file_handle)) {
            return;
        }
    }

    file_handle_probe = file_handle;
    storage_cursor_probe = 0;
    do {
        request_bytes = (u16)XMS_READ_BYTES;
        difference = source_remaining_probe - XMS_READ_BYTES;
        if ((i32)difference < 0) {
            request_bytes = (u16)source_remaining_probe;
        }

        bytes_read = dos_read_probe(
                file_handle, staging_buffer, request_bytes);
        source_remaining_probe -= (u32)bytes_read;

        move_bytes = bytes_read;
        if ((bytes_read & 1u) != 0) {
            ++move_bytes;
        }
        xms_move_request_shared_probe.length = (u32)move_bytes;
        xms_move_request_shared_probe.source_handle = 0;
        xms_move_request_shared_probe.source.pointer = staging_buffer;
        xms_move_request_shared_probe.destination_handle = (u16)xms_handle_probe;
        xms_move_request_shared_probe.destination.offset = storage_cursor_probe;
        storage_cursor_probe += XMS_READ_BYTES;
        xms_move_probe(&xms_move_request_shared_probe);
    } while (source_remaining_probe != 0);

    if ((path_is_embedded_probe & 1u) == 0) {
        dos_close_probe(file_handle);
    }
    archive_size_probe = archive_remaining_probe;
}
