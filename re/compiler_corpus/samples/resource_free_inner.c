/* Codegen probe for BLOODPRG 0x00529C. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct resource_handle_probe {
    u16 segment;
    u16 flags;
    u32 size;
} resource_handle_probe;

extern volatile resource_handle_probe resource_handle_table_probe[];
extern volatile u16 resident_handles_probe[256];
extern volatile u32 free_bytes_probe;
extern volatile u16 pool_end_segment_probe;
void FAR far_memmove_probe(volatile u8 FAR *destination,
        const volatile u8 FAR *source, u32 byte_count);

#if defined(__WATCOMC__)
#pragma aux resource_free_inner_probe parm [ax] modify exact []
#endif

void FAR resource_free_inner_probe(u16 handle)
{
    volatile resource_handle_probe *released;
    volatile resource_handle_probe *shifted_entry;
    volatile u8 FAR *destination;
    const volatile u8 FAR *source;
    u32 released_size;
    u32 moved_bytes;
    u16 released_segment;
    u16 paragraphs;
    u16 position;
    u16 shifted_handle;

    released = &resource_handle_table_probe[handle];
    released_segment = released->segment;
    released->flags &= (u16)~3u;
    released_size = released->size;
    free_bytes_probe += released_size;
    paragraphs = (u16)(released_size >> 4);
    pool_end_segment_probe = (u16)(pool_end_segment_probe - paragraphs);

    position = 0u;
    while (resident_handles_probe[position] != handle) {
        ++position;
    }

    moved_bytes = 0u;
    do {
        shifted_handle = resident_handles_probe[position + 1u];
        resident_handles_probe[position] = shifted_handle;
        if ((i16)shifted_handle < 0) {
            break;
        }

        shifted_entry = &resource_handle_table_probe[shifted_handle];
        shifted_entry->segment =
                (u16)(shifted_entry->segment - paragraphs);
        moved_bytes += shifted_entry->size;
        ++position;
    } while (1);

    if (moved_bytes != 0u) {
        destination = (volatile u8 FAR *)((u32)released_segment << 16);
        source = (const volatile u8 FAR *)
                ((u32)(u16)(released_segment + paragraphs) << 16);
        far_memmove_probe(destination, source, moved_bytes);
    }
}
