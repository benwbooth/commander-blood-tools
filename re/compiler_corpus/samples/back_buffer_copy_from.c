/*
 * Codegen probe for BLOODPRG 0x00933A.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile u8 FAR *graphics_buffer_ptr;

extern graphics_buffer_ptr GAME_DATA work_surface;
extern graphics_buffer_ptr GAME_DATA back_buffer;
void FAR *NEAR _fmemcpy(void FAR *destination, const void FAR *source, u16 count);

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemcpy)
#pragma aux back_buffer_copy_from_probe parm [bx] [cx] [dx] modify exact []
#endif

void NEAR back_buffer_copy_from_probe(u16 x, u16 y, u16 width)
{
    graphics_buffer_ptr source;
    graphics_buffer_ptr destination;
    u16 offset;

    offset = (u16)(y * 320u + x);
    source = work_surface + offset;
    destination = back_buffer + offset;
    _fmemcpy((void FAR *)destination, (const void FAR *)source, width);
}
