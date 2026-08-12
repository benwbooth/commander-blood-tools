/*
 * Codegen probe for BLOODPRG 0x00A38E.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_wrap_limit;
extern u16 list_d8c_iteration_count;
extern volatile u16 list_d8c_wrap_count;
extern volatile u16 list_d8c_buffer_end_offset;

void NEAR queue_d8c_wrap_probe(u16 byte_count, u16 cursor)
{
    u16 next;

    next = (u16)(cursor + byte_count);
    if (next < cursor || next > list_d8c_buffer_end_offset) {
        u16 old_head;

        old_head = list_d8c_head_offset;
        list_d8c_head_offset = 0;
        list_d8c_wrap_limit = old_head;
    }

    list_d8c_iteration_count = (u16)(byte_count - 2u);
    ++list_d8c_wrap_count;
}
