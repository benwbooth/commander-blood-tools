/*
 * Codegen probe for BLOODPRG 0x002ABB.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed long i32;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define READ_BYTES 0x7d00UL

typedef struct dos_dta_probe {
    u8 reserved_00[0x1a];
    u32 file_size;
} dos_dta_probe;

extern volatile u32 archive_size_probe;
extern volatile u32 source_remaining_probe;
extern volatile u8 path_is_embedded_probe;
extern volatile u16 file_handle_probe;

u16 FAR resource_source_select_probe(volatile char FAR *path);
volatile dos_dta_probe FAR *NEAR dos_get_dta_probe(void);
int NEAR dos_find_first_probe(const volatile char FAR *path);
int NEAR dos_open_read_only_probe(
        const volatile char FAR *path, u16 *handle);
u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
void NEAR dos_close_probe(u16 handle);

u32 FAR resource_file_load_probe(volatile char FAR *path,
        volatile u8 FAR *destination)
{
    volatile dos_dta_probe FAR *dta;
    u32 difference;
    u16 handle;
    u16 request_bytes;
    u16 bytes_read;
    u16 destination_segment;
    u16 destination_offset;
    int embedded;

    handle = resource_source_select_probe(path);
    embedded = (path_is_embedded_probe & 1u) != 0;
    if (!embedded) {
        dta = dos_get_dta_probe();
        (void)dos_find_first_probe(path);
        archive_size_probe = dta->file_size;
        source_remaining_probe = dta->file_size;
        if (!dos_open_read_only_probe(path, &handle)) {
            return 0;
        }
    }

    file_handle_probe = handle;
    do {
        request_bytes = (u16)READ_BYTES;
        difference = source_remaining_probe - READ_BYTES;
        if ((i32)difference < 0) {
            request_bytes = (u16)source_remaining_probe;
        }

        bytes_read = dos_read_probe(handle, destination, request_bytes);
        source_remaining_probe -= (u32)bytes_read;

        destination_segment = (u16)(FP_SEG(destination) + (bytes_read >> 4));
        destination_offset = (u16)(FP_OFF(destination) + (bytes_read & 0x0fu));
        destination = (volatile u8 FAR *)MK_FP(
                destination_segment, destination_offset);
    } while (source_remaining_probe != 0);

    if (!embedded) {
        dos_close_probe(handle);
    }
    return archive_size_probe;
}
