/*
 * Codegen probe for BLOODPRG 0x00280F.
 * This is not recovered game source.
 */
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

typedef volatile u8 FAR *resource_buffer_ptr_probe;

extern resource_buffer_ptr_probe resource_copy_buffer_probe;
extern volatile u16 resource_copy_file_handle_probe;

u32 FAR resource_name_lookup_probe(volatile char FAR *filename);
int NEAR dos_open_read_only_probe(
        const volatile char FAR *path, u16 *handle);
int NEAR dos_create_truncate_probe(
        const volatile char FAR *path, u16 *handle);
u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
u16 NEAR dos_write_probe(
        u16 handle, const volatile u8 FAR *source, u16 byte_count);
void NEAR dos_close_probe(u16 handle);

void FAR startup_resource_file_copy_probe(
        volatile char FAR *source_path,
        const volatile char FAR *destination_path)
{
    volatile u8 FAR *buffer;
    u32 remaining;
    u16 source_handle;
    u16 destination_handle;
    u16 bytes_read;

    remaining = resource_name_lookup_probe(source_path);
    if (remaining == 0) {
        return;
    }
    if (!dos_open_read_only_probe(source_path, &source_handle)) {
        return;
    }

    resource_copy_file_handle_probe = source_handle;
    if (!dos_create_truncate_probe(destination_path, &destination_handle)) {
        return;
    }

    buffer = resource_copy_buffer_probe;
    do {
        resource_copy_file_handle_probe = destination_handle;
        bytes_read = dos_read_probe(source_handle, buffer, 0xfa00u);
        remaining -= (u32)bytes_read;

        resource_copy_file_handle_probe = source_handle;
        (void)dos_write_probe(destination_handle, buffer, bytes_read);
    } while (remaining != 0);

    dos_close_probe(destination_handle);
    dos_close_probe(source_handle);
}
