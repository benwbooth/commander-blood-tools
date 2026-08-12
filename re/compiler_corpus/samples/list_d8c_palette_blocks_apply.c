/* Codegen probe for BLOODPRG 0x00A778. */

typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 FAR list_d8c_buffer[];
extern volatile u16 list_d8c_palette_offset;
extern volatile u8 FAR *NEAR resource_palette_blocks_apply_probe(
        volatile u8 FAR *stream);

#if defined(__WATCOMC__)
#pragma aux resource_palette_blocks_apply_probe \
        parm [es si] value [es si] modify [si]
#endif

volatile u8 FAR *NEAR list_d8c_palette_blocks_apply_probe(void);

#if defined(__WATCOMC__)
#pragma aux list_d8c_palette_blocks_apply_probe \
        value [es si] modify [si]
#endif

volatile u8 FAR *NEAR list_d8c_palette_blocks_apply_probe(void)
{
    return resource_palette_blocks_apply_probe(
            list_d8c_buffer + list_d8c_palette_offset);
}
