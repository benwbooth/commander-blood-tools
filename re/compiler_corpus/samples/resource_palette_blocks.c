/*
 * Codegen probe for BLOODPRG 0x00A0C3.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 palette_dirty_probe;
extern volatile u8 live_palette_probe[768];
extern volatile u16 read_wrap_index_probe;
extern u16 entry_metric_probe;
void NEAR render_state_copy_gate_probe(void);

volatile u8 FAR *NEAR resource_palette_blocks_probe(volatile u8 FAR *stream)
{
    volatile u8 FAR *cursor;
    volatile u8 FAR *start;
    volatile u8 *destination;
    u16 consumed;
    u16 header;
    u16 byte_count;
    u16 remaining;

    start = stream;
    cursor = stream;
    palette_dirty_probe = 1;

    for (;;) {
        header = *(volatile u16 FAR *)cursor;
        cursor += 2;
        if (header == 0xffffu) {
            break;
        }

        destination = live_palette_probe + (u16)((header & 0x00ffu) * 3u);
        byte_count = (u16)((header >> 8) * 3u);
        while (byte_count != 0) {
            *destination++ = *cursor++;
            --byte_count;
        }
    }

    render_state_copy_gate_probe();
    if (read_wrap_index_probe == 0) {
        consumed = (u16)(cursor - start);
        remaining = (u16)(entry_metric_probe - consumed);
        entry_metric_probe = (u16)((remaining >> 2) - 2u);
    }

    return cursor;
}
