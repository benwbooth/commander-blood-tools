/*
 * Codegen probe for BLOODPRG 0x00A757.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

extern volatile u16 list_d8c_base_segment;
extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_head_segment;
extern volatile u16 list_d8c_tail_offset;
extern volatile u16 list_d8c_tail_segment;
extern volatile u16 list_d8c_byte_count;
extern u16 list_d8c_iteration_count;
extern volatile u16 list_d8c_active_offset;
extern volatile u16 list_d8c_wrap_limit;
extern volatile u16 list_d8c_buffer_end_offset;

void FAR list_d8c_init_probe(void)
{
    u16 base_segment;

    base_segment = list_d8c_base_segment;
    list_d8c_head_segment = base_segment;
    list_d8c_tail_segment = base_segment;

    list_d8c_head_offset = 0;
    list_d8c_tail_offset = 0;
    list_d8c_byte_count = 0;
    list_d8c_iteration_count = 0;
    list_d8c_active_offset = 0;
    list_d8c_wrap_limit = list_d8c_buffer_end_offset;
}
