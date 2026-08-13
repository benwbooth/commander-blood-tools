/*
 * Codegen probe for BLOODPRG 0x004086.
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

extern volatile u8 palette_dirty;
extern volatile u8 live_palette[];

u16 NEAR dos_read_probe(u16 handle,
        volatile u8 FAR *destination, u16 byte_count);

void NEAR resource_palette_file_blocks_probe(u16 file_handle,
        volatile u16 *header_buffer,
        u32 *remaining_bytes)
{
    volatile u8 *destination;
    u16 header;
    u16 byte_count;

    palette_dirty = 1;
    for (;;) {
        (void)dos_read_probe(file_handle,
                (volatile u8 FAR *)header_buffer, 2u);
        header = *header_buffer;
        *remaining_bytes -= 2u;
        if (header == 0xffffu) {
            break;
        }
        destination = live_palette + (u16)((header & 0x00ffu) * 3u);
        byte_count = (u16)((header >> 8) * 3u);
        *remaining_bytes -= byte_count;
        (void)dos_read_probe(file_handle,
                (volatile u8 FAR *)destination, byte_count);
    }
}
