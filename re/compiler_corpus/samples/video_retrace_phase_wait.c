/* Codegen probe for BLOODPRG 0x000BD7. */

#include <conio.h>

typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

extern volatile u16 video_crtc_base_port;
extern volatile u8 video_retrace_phase;

void FAR video_retrace_phase_wait_probe(void)
{
    u8 phase;
    u8 expected;
    u16 status_port;

    phase = video_retrace_phase;
    if (phase == 0u) {
        return;
    }

    status_port = (u16)(video_crtc_base_port + 6u);
    expected = phase == 1u ? 0x08u : 0u;
    while (((u8)inp(status_port) & 0x08u) == expected) {
    }
}
