/*
 * Codegen probe for BLOODPRG 0x00A40B.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 list_d8c_state_byte;

int NEAR list_d8c_state_le_one_probe(void)
{
    if (list_d8c_state_byte == 0) {
        return 1;
    }
    return list_d8c_state_byte == 1;
}
