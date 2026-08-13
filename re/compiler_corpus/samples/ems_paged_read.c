/* Codegen probe for BLOODPRG 0x00A664. */
typedef unsigned char u8;
typedef signed short i16;
typedef unsigned short u16;
typedef unsigned long u32;

#if defined(__WATCOMC__)
#define FAR __far
#define NEAR __near
#elif defined(__TURBOC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

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

extern volatile u8 resource_source_is_banked;
extern volatile i16 resource_xms_handle;
extern volatile i16 resource_ems_handle;
extern volatile u32 resource_source_offset;
extern volatile u32 resource_source_remaining;
extern volatile u16 list_d8c_file_handle;
extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_byte_count;
extern volatile u8 FAR list_d8c_buffer[];
extern volatile u8 FAR ems_page_frame[];
extern volatile xms_move_request shared_xms_move_request;

extern void NEAR cb_ems_map_page_probe(u16 handle, u16 logical_page,
        u8 physical_page);
extern void NEAR cb_xms_move_probe(volatile xms_move_request *request);
extern void FAR far_memmove_probe(volatile u8 FAR *destination,
        const volatile u8 FAR *source, u32 byte_count);
extern void NEAR cb_dos_seek_absolute_probe(u16 handle, u32 offset);
extern u16 NEAR cb_dos_read_probe(u16 handle,
        volatile u8 FAR *destination, u16 byte_count);

int NEAR ems_paged_read_probe(u16 byte_count)
{
    u32 source_offset;
    u16 logical_page;
    u16 transferred;
    u16 handle;
    u8 physical_page;

    if ((resource_source_is_banked & 1u) != 0
            && resource_ems_handle != -1) {
        source_offset = resource_source_offset;
        logical_page = (u16)(source_offset >> 14);
        handle = (u16)resource_ems_handle;
        for (physical_page = 0; physical_page < 4u; ++physical_page) {
            cb_ems_map_page_probe(handle, logical_page, physical_page);
            ++logical_page;
        }
        far_memmove_probe(
                list_d8c_buffer + list_d8c_head_offset,
                ems_page_frame + (u16)(source_offset & 0x3fffu),
                byte_count);
        transferred = byte_count;
    } else if ((resource_source_is_banked & 1u) != 0
            && resource_xms_handle != -1) {
        shared_xms_move_request.length =
                (u32)byte_count + (byte_count & 1u);
        shared_xms_move_request.source_handle = (u16)resource_xms_handle;
        shared_xms_move_request.source.offset = resource_source_offset;
        shared_xms_move_request.destination_handle = 0;
        shared_xms_move_request.destination.pointer =
                list_d8c_buffer + list_d8c_head_offset;
        cb_xms_move_probe(&shared_xms_move_request);
        transferred = byte_count;
    } else {
        handle = list_d8c_file_handle;
        if (handle < 1u) {
            return 0;
        }
        do {
            cb_dos_seek_absolute_probe(handle, resource_source_offset);
            transferred = cb_dos_read_probe(
                    handle,
                    list_d8c_buffer + list_d8c_head_offset,
                    byte_count);
        } while (transferred < byte_count);
    }

    resource_source_remaining -= transferred;
    resource_source_offset += transferred;
    list_d8c_head_offset += transferred;
    list_d8c_byte_count += transferred;
    return 1;
}
