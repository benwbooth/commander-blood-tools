/*
 * Codegen probe for BLOODPRG 0x00BD26.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#include <string.h>
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
void FAR *NEAR _fmemcpy(void FAR *destination,
        const void FAR *source, u16 count);
#endif

extern volatile i16 snd_bank_ems_handle;
extern volatile u16 ems_page_frame_segment;
void NEAR cb_ems_map_page_probe(u16 handle, u16 logical_page,
        u8 physical_page);

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemcpy)
#pragma aux snd_bank_ems_page_read_probe parm [ax] [es di] modify exact []
#endif

void NEAR snd_bank_ems_page_read_probe(u16 page,
        volatile u8 FAR *destination)
{
    u16 page_frame;
    u16 handle;

    page_frame = ems_page_frame_segment;
    handle = (u16)snd_bank_ems_handle;
    cb_ems_map_page_probe(handle, page, 0u);
    _fmemcpy(
            (void FAR *)destination,
            (const void FAR *)MK_FP(page_frame, 0u),
            0x4000u);
}
