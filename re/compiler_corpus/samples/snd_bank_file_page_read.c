/*
 * Codegen probe for BLOODPRG 0x00BD8D.
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

extern volatile u16 snd_bank_file_handle;
void NEAR cb_dos_seek_absolute_probe(u16 handle, u32 offset);
u16 NEAR cb_dos_read_probe(u16 handle,
        volatile u8 FAR *destination, u16 byte_count);

#if defined(__WATCOMC__)
#pragma aux snd_bank_file_page_read_probe parm [ax] [es di] modify exact []
#endif

void NEAR snd_bank_file_page_read_probe(u16 page,
        volatile u8 FAR *destination)
{
    u32 offset;
    u16 handle;

    offset = (u32)page << 14;
    handle = snd_bank_file_handle;
    cb_dos_seek_absolute_probe(handle, offset);
    (void)cb_dos_read_probe(handle, destination, 0x4000u);
}
