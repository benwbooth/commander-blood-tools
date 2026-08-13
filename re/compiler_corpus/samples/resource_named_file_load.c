/*
 * Codegen probe for BLOODPRG 0x003FC7.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct resource_name_entry_probe {
    char filename[16];
} resource_name_entry_probe;

typedef struct dos_dta_probe {
    u8 reserved[0x1a];
    u32 file_size;
} dos_dta_probe;

typedef struct allocation_result_probe {
    i16 status;
    volatile u8 FAR *destination;
} allocation_result_probe;

extern volatile resource_name_entry_probe resource_names_probe[];
extern volatile u16 resource_file_header_probe;

u16 FAR path_builder_probe(volatile char *filename);
volatile dos_dta_probe FAR *dos_get_dta_probe(void);
int dos_find_first_probe(const volatile char *filename);
int dos_open_read_only_probe(const volatile char *filename, u16 *handle);
u16 dos_read_probe(u16 handle, volatile u8 FAR *destination, u16 count);
void dos_close_probe(u16 handle);
void palette_file_blocks_probe(u16 handle, volatile u16 *header, u32 *remaining);
allocation_result_probe FAR allocate_probe(u16 handle, u32 byte_count);

int FAR resource_named_file_load_probe(u16 resource_id,
        volatile u8 FAR *direct_destination)
{
    allocation_result_probe allocation;
    volatile dos_dta_probe FAR *dta;
    volatile char *filename;
    volatile u8 FAR *destination;
    u32 remaining;
    u16 handle;
    u16 header;
    u16 bytes_read;

    filename = resource_names_probe[resource_id].filename;
    (void)path_builder_probe(filename);
    dta = dos_get_dta_probe();
    if (!dos_find_first_probe(filename)) {
        return -1;
    }
    remaining = dta->file_size;
    if (!dos_open_read_only_probe(filename, &handle)) {
        return -1;
    }
    (void)dos_read_probe(handle,
            (volatile u8 FAR *)&resource_file_header_probe, 2u);
    header = resource_file_header_probe;
    if (header & 2u) {
        palette_file_blocks_probe(handle, &resource_file_header_probe,
                &remaining);
    }
    destination = direct_destination;
    if ((i16)resource_id >= 0) {
        allocation = allocate_probe(resource_id, remaining);
        if (allocation.status < 0) {
            dos_close_probe(handle);
            return -1;
        }
        if (allocation.status != 0) {
            dos_close_probe(handle);
            return 0;
        }
        destination = allocation.destination;
    }
    *(volatile u16 FAR *)destination = header;
    destination += 2u;
    remaining -= 2u;
    for (;;) {
        bytes_read = dos_read_probe(handle, destination, 0x7d00u);
        remaining -= bytes_read;
        if (remaining == 0) {
            break;
        }
        destination = (volatile u8 FAR *)MK_FP(
                (u16)(FP_SEG(destination) + (bytes_read >> 4)),
                (u16)(FP_OFF(destination) + (bytes_read & 0x000fu)));
    }
    dos_close_probe(handle);
    return 0;
}
