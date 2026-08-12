/*
 * Codegen probe for BLOODPRG 0x00A3D0.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u16 FAR *list_d8c_tail_pointer;
extern volatile u16 list_d8c_tail_offset;
extern volatile u16 list_d8c_byte_count;
extern volatile u16 list_d8c_buffer_end_offset;
extern volatile u16 list_d8c_sequence_index;
extern volatile u16 list_d8c_read_wrap_index;
extern volatile u16 list_d8c_read_wrap_limit;

void NEAR queue_d8c_consume_probe(void)
{
    u16 entry_bytes;
    u16 after_header;
    u16 candidate;
    u16 next_index;

    entry_bytes = *list_d8c_tail_pointer;
    list_d8c_byte_count = (u16)(list_d8c_byte_count - entry_bytes);

    after_header = (u16)(list_d8c_tail_offset + 2u);
    candidate = (u16)(after_header + entry_bytes);
    if (candidate < after_header || candidate > list_d8c_buffer_end_offset) {
        list_d8c_tail_offset = (u16)(entry_bytes - 2u);
    } else {
        list_d8c_tail_offset = (u16)(list_d8c_tail_offset + entry_bytes);
    }

    ++list_d8c_sequence_index;
    next_index = (u16)(list_d8c_read_wrap_index + 1u);
    if (next_index > list_d8c_read_wrap_limit) {
        next_index = 1u;
        list_d8c_read_wrap_limit = 0xffffu;
    }
    list_d8c_read_wrap_index = next_index;
}
