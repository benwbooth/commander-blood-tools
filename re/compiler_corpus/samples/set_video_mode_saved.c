/* Codegen probe for BLOODPRG 0x000CC0. */

#include <dos.h>

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

extern volatile unsigned char saved_video_mode;

void FAR set_video_mode_saved_probe(void)
{
    union REGS registers;

    registers.h.ah = 0;
    registers.h.al = saved_video_mode;
    int86(0x10, &registers, &registers);
}
