/*
 * Codegen probe for BLOODPRG 0x00BD09.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 snd_bank_storage_mode;

void NEAR snd_bank_ems_page_read(u16 page, volatile u8 FAR *destination);
void NEAR snd_bank_xms_page_read(u16 page, volatile u8 FAR *destination);
void NEAR snd_bank_file_page_read(u16 page, volatile u8 FAR *destination);

#if defined(__WATCOMC__)
#pragma aux snd_bank_ems_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_xms_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_file_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_page_read_probe parm [ax] [es di]
#endif

void NEAR snd_bank_page_read_probe(u16 page,
        volatile u8 FAR *destination)
{
    i8 mode;

    mode = (i8)snd_bank_storage_mode;
    if (--mode < 0) {
        snd_bank_ems_page_read(page, destination);
    } else {
        if (--mode < 0) {
            snd_bank_xms_page_read(page, destination);
        } else {
            snd_bank_file_page_read(page, destination);
        }
    }
}
