/*
 * Codegen probe for BLOODPRG 0x00A744.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 list_d8c_wrap_count;
extern volatile u16 list_d8c_read_wrap_limit;
extern volatile u16 list_d8c_secondary_wrap_limit;

void NEAR list_d8c_wrap_bounds_reset_probe(void)
{
    list_d8c_wrap_count = 0;
    list_d8c_read_wrap_limit = 0xffffu;
    list_d8c_secondary_wrap_limit = 0xffffu;
}
