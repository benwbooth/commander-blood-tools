#include <conio.h>

#include "../include/bloodprg_hardware.h"

void CB_FAR vga_retrace_phase_calibrate(void)
{
    cb_u16 status_port;
    cb_u16 first_elapsed;
    cb_u16 second_elapsed;
    cb_u16 second_width;
    cb_u16 timer_count;
    cb_u8 edge_reference;
    cb_u8 first_phase_set;

    status_port = (cb_u16)(video_crtc_base_port + 6u);
    edge_reference = (cb_u8)(inp(status_port) & 0x08u);
    video_calibration_ticks = 2u;

    while ((video_calibration_ticks & 3u) != 0u) {
        if (((cb_u8)inp(status_port) & 0x08u) == edge_reference) {
            continue;
        }

        ++video_retrace_phase;
        outp(0x61u, inp(0x61u) | 0x01u);
        outp(0x43u, 0xb0u);
        outp(0x42u, 0xffu);
        outp(0x42u, 0xffu);

        edge_reference = (cb_u8)(inp(status_port) & 0x08u);
        first_phase_set = edge_reference != 0u;
        while (((cb_u8)inp(status_port) & 0x08u) == edge_reference) {
        }

        outp(0x43u, 0x80u);
        timer_count = (cb_u8)inp(0x42u);
        timer_count = (cb_u16)(
                timer_count | ((cb_u16)(cb_u8)inp(0x42u) << 8));
        first_elapsed = (cb_u16)(0u - timer_count);

        edge_reference = (cb_u8)(inp(status_port) & 0x08u);
        while (((cb_u8)inp(status_port) & 0x08u) == edge_reference) {
        }

        outp(0x43u, 0x80u);
        timer_count = (cb_u8)inp(0x42u);
        timer_count = (cb_u16)(
                timer_count | ((cb_u16)(cb_u8)inp(0x42u) << 8));
        second_elapsed = (cb_u16)(0u - timer_count);
        second_width = (cb_u16)(second_elapsed - first_elapsed);

        if (((cb_i16)second_width > (cb_i16)first_elapsed) !=
            (first_phase_set != 0u)) {
            break;
        }
        ++video_retrace_phase;
        break;
    }

    timer_reload_ticks = 3u;
}
