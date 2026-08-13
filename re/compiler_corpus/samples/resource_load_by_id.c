/*
 * Codegen probe for BLOODPRG 0x00287B.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define FS_DATA __based(__segname("FS_DATA"))
#else
#define FS_DATA FAR
#endif

typedef struct resource_name_entry_probe {
    char filename[16];
} resource_name_entry_probe;

typedef struct allocation_result_probe {
    i16 status;
    volatile u8 FAR *destination;
} allocation_result_probe;

extern volatile resource_name_entry_probe FS_DATA resource_names_probe[];

u32 FAR resource_name_lookup_probe(const volatile char FAR *filename);
allocation_result_probe FAR resource_allocate_probe(
        u16 resource_id, u32 byte_count);
u32 FAR resource_file_load_probe(const volatile char FAR *filename,
        volatile u8 FAR *destination);

#if defined(__WATCOMC__)
#pragma aux resource_load_by_id_probe parm [ax] value [ax] modify exact [ax]
#endif

int FAR resource_load_by_id_probe(u16 resource_id)
{
    allocation_result_probe allocation;
    const volatile char FS_DATA *resource_name;
    const volatile char FAR *filename;
    u32 byte_count;

    resource_name = resource_names_probe[resource_id].filename;
    filename = resource_name;
    byte_count = resource_name_lookup_probe(filename);
    if (byte_count == 0) {
        return 0;
    }

    allocation = resource_allocate_probe(resource_id, byte_count);
    if (allocation.status < 0) {
        return 0;
    }
    if (allocation.status != 0) {
        return 1;
    }

    return resource_file_load_probe(filename, allocation.destination) != 0;
}
