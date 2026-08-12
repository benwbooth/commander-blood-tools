/*
 * Codegen probe for BLOODPRG 0x00A2DD.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 list_d8c_state_byte;
extern volatile u16 list_d8c_byte_count;
void NEAR close_file_d5b_probe(void);

void NEAR presentation_queue_finish_probe(void)
{
    list_d8c_state_byte |= 1u;
    if (list_d8c_byte_count == 0) {
        list_d8c_state_byte |= 2u;
        close_file_d5b_probe();
    }
}
