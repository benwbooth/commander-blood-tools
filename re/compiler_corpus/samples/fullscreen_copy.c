/*
 * Codegen probe for BLOODPRG 0x003E46/0x003E5B.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <string.h>
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
void FAR *NEAR _fmemcpy(void FAR *destination,
        const void FAR *source, u16 count);
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#pragma intrinsic(_fmemcpy)
#else
#define GAME_DATA FAR
#endif

typedef volatile u8 FAR *buffer_pointer;
extern buffer_pointer GAME_DATA display_buffer;

void FAR fullscreen_copy_probe(const u32 FAR *source);

#if defined(__WATCOMC__)
#pragma aux fullscreen_copy_probe parm [ds si] modify exact []
#endif

void FAR fullscreen_copy_probe(const u32 FAR *source)
{
#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    _fmemcpy((void FAR *)display_buffer,
            (const void FAR *)source, 0xfa00u);

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
