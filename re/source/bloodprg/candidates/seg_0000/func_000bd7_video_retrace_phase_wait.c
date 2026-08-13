#include <conio.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR video_retrace_phase_wait(void)
{
    cb_u8 phase;
    cb_u8 expected;
    cb_u16 status_port;

    phase = video_retrace_phase;
    if (phase == 0u) {
        return;
    }

    status_port = (cb_u16)(video_crtc_base_port + 6u);
    expected = phase == 1u ? 0x08u : 0u;
    while (((cb_u8)inp(status_port) & 0x08u) == expected) {
    }
}
