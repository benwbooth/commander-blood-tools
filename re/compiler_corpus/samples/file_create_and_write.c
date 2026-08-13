/*
 * Codegen probe for BLOODPRG 0x002B6B.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define WRITE_BYTES 0x7d00u

extern volatile u32 remaining_probe;
extern volatile u16 file_handle_probe;

void FAR write_directory_enter_probe(void);
int NEAR dos_create_truncate_probe(
        const volatile char FAR *path, u16 *handle);
u16 NEAR dos_write_probe(u16 handle,
        const volatile u8 FAR *source, u16 byte_count);
void NEAR dos_close_probe(u16 handle);

u32 FAR file_create_and_write_probe(
        const volatile char FAR *path,
        const volatile u8 FAR *source,
        u32 byte_count)
{
    u16 file_handle;
    u16 request_bytes;
    u16 bytes_written;
    u16 source_segment;
    u16 source_offset;

    write_directory_enter_probe();
    remaining_probe = byte_count;

    if (!dos_create_truncate_probe(path, &file_handle)) {
        return 0;
    }
    file_handle_probe = file_handle;

    do {
        request_bytes = (u16)remaining_probe;
        if ((u16)(remaining_probe >> 16) != 0) {
            request_bytes = WRITE_BYTES;
        }

        bytes_written = dos_write_probe(
                file_handle, source, request_bytes);
        remaining_probe -= (u32)bytes_written;

        source_segment = (u16)(FP_SEG(source) + (bytes_written >> 4));
        source_offset = (u16)(FP_OFF(source) + (bytes_written & 0x0fu));
        source = (const volatile u8 FAR *)MK_FP(
                source_segment, source_offset);
    } while (remaining_probe != 0);

    dos_close_probe(file_handle);
    return byte_count;
}
