/*
 * Codegen probe for BLOODPRG 0x00A734.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_byte_count;

void NEAR queue_d8c_enqueue_probe(u16 byte_count)
{
    list_d8c_head_offset += byte_count;
    list_d8c_byte_count += byte_count;
}
