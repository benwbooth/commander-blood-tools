/*
 * Codegen probe for BLOODPRG 0x009F8E.
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

typedef struct resource_descriptor_probe {
    u8 flags;
    u8 variant;
    char filename[1];
} resource_descriptor_probe;

typedef struct dos_dta_probe {
    u8 reserved_00[0x1a];
    u32 file_size;
} dos_dta_probe;

extern volatile u16 requested_id_probe;
extern volatile u16 active_id_probe;
extern volatile u16 resource_flags_probe;
extern volatile u32 range_start_probe;
extern volatile u32 range_remaining_probe;
extern volatile u32 index_start_probe;
extern volatile u32 index_remaining_probe;
extern volatile u32 source_offset_probe;
extern volatile u32 source_remaining_probe;
extern volatile u32 archive_size_probe;
extern volatile u32 archive_offset_probe;
extern volatile u32 archive_remaining_probe;
extern volatile u8 path_is_embedded_probe;
extern volatile u8 source_is_banked_probe;
extern volatile u8 resource_marker_probe;
extern volatile u8 variant_probe;
extern volatile char path_buffer_probe[];
extern volatile u8 list_state_probe;
extern volatile u16 list_byte_count_probe;
extern volatile u16 list_head_offset_probe;
extern volatile u16 list_entry_metric_probe;
extern volatile u16 list_buffer_end_probe;
extern volatile u16 file_handle_probe;
extern volatile u8 FAR list_buffer_probe[];

void NEAR close_file_probe(void);
void FAR list_init_probe(void);
void NEAR list_bounds_init_probe(void);
resource_descriptor_probe *NEAR descriptor_lookup_probe(u16 resource_id);
u16 FAR path_builder_probe(volatile char FAR *filename);
volatile dos_dta_probe FAR *NEAR dos_get_dta_probe(void);
void NEAR dos_find_first_probe(const volatile char FAR *path);
int NEAR dos_open_read_only_probe(const volatile char FAR *path, u16 *handle);
int NEAR list_read_probe(u16 *entry_extent, u16 *cursor_offset);
int NEAR paged_read_probe(u16 byte_count);
volatile u8 FAR *NEAR palette_blocks_probe(volatile u8 FAR *stream);

#if defined(__WATCOMC__)
#pragma aux descriptor_lookup_probe parm [ax] value [bx] modify [bx]
#endif

int NEAR resource_switch_probe(u16 resource_id)
{
    resource_descriptor_probe *descriptor;
    volatile dos_dta_probe FAR *dta;
    volatile u8 FAR *stream;
    u16 saved_byte_count;
    u16 saved_head_offset;
    u16 entry_extent;
    u16 cursor_offset;
    u16 end_offset;
    u16 byte_count;
    u16 handle;
    u16 table_offset;
    u32 relative_offset;
    int read_succeeded;

    requested_id_probe = resource_id;
    close_file_probe();
    list_init_probe();
    list_state_probe = 0;
    list_bounds_init_probe();

    active_id_probe = resource_id;
    descriptor = descriptor_lookup_probe(resource_id);
    descriptor->variant = variant_probe;
    resource_flags_probe = (u16)descriptor->flags
            | ((u16)descriptor->variant << 8);

    source_remaining_probe = archive_size_probe;
    handle = 0;
    source_offset_probe = 0;

    if ((source_is_banked_probe & 1u) == 0) {
        handle = path_builder_probe(descriptor->filename);
        source_remaining_probe = archive_remaining_probe;
        source_offset_probe = archive_offset_probe;

        if ((path_is_embedded_probe & 1u) == 0) {
            dta = dos_get_dta_probe();
            dos_find_first_probe(path_buffer_probe);
            source_remaining_probe = dta->file_size;
            if (!dos_open_read_only_probe(path_buffer_probe, &handle)) {
                file_handle_probe = handle;
                return 0;
            }
            source_offset_probe = 0;
        }
    }
    file_handle_probe = handle;

    saved_byte_count = list_byte_count_probe;
    saved_head_offset = list_head_offset_probe;
    read_succeeded = list_read_probe(&entry_extent, &cursor_offset);
    if (read_succeeded) {
        list_entry_metric_probe = entry_extent;
        end_offset = (u16)(cursor_offset + entry_extent);
        if (end_offset < cursor_offset || end_offset > list_buffer_end_probe) {
            list_head_offset_probe = 0;
        }

        byte_count = (u16)(entry_extent - 2u);
        read_succeeded = paged_read_probe(byte_count);
    }
    list_head_offset_probe = saved_head_offset;
    list_byte_count_probe = saved_byte_count;
    if (!read_succeeded) {
        return 0;
    }

    stream = list_buffer_probe + list_head_offset_probe;
    entry_extent = *(volatile u16 FAR *)stream;
    stream += 2;
    cursor_offset = (u16)(list_head_offset_probe + 2u);
    end_offset = (u16)(cursor_offset + entry_extent);
    if (end_offset < cursor_offset || end_offset > list_buffer_end_probe) {
        stream = list_buffer_probe;
    }

    resource_marker_probe = 0xffu;
    stream = palette_blocks_probe(stream);
    while (*stream == 0xffu) {
        ++stream;
    }

    table_offset = (resource_flags_probe & 0x0004u) != 0 ? 0x10u : 0;
    relative_offset = *(volatile u32 FAR *)(stream + table_offset);
    range_start_probe = source_offset_probe + relative_offset;
    range_remaining_probe = source_remaining_probe - relative_offset;

    table_offset = (u16)(list_entry_metric_probe * 4u);
    relative_offset = *(volatile u32 FAR *)(stream + table_offset);
    index_start_probe = source_offset_probe + relative_offset;
    index_remaining_probe = source_remaining_probe - relative_offset;

    return 1;
}
