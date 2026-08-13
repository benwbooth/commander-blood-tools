/* Codegen probe for BLOODPRG 0x000B42. */

#include <conio.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA
#endif

extern volatile u16 GAME_DATA crtc_base_port;
extern volatile u8 GAME_DATA retrace_phase;
extern volatile u16 GAME_DATA calibration_ticks;
extern volatile u16 GAME_DATA timer_reload_ticks;

void far vga_retrace_phase_calibrate_probe(void)
{
    u16 status_port;
    u16 first_elapsed;
    u16 second_elapsed;
    u16 second_width;
    u16 timer_count;
    u8 edge_reference;
    u8 first_phase_set;

    status_port = (u16)(crtc_base_port + 6u);
    edge_reference = (u8)(inp(status_port) & 0x08u);
    calibration_ticks = 2u;

    while ((calibration_ticks & 3u) != 0u) {
        if (((u8)inp(status_port) & 0x08u) == edge_reference) {
            continue;
        }

        ++retrace_phase;
        outp(0x61u, inp(0x61u) | 0x01u);
        outp(0x43u, 0xb0u);
        outp(0x42u, 0xffu);
        outp(0x42u, 0xffu);

        edge_reference = (u8)(inp(status_port) & 0x08u);
        first_phase_set = edge_reference != 0u;
        while (((u8)inp(status_port) & 0x08u) == edge_reference) {
        }

        outp(0x43u, 0x80u);
        timer_count = (u8)inp(0x42u);
        timer_count = (u16)(timer_count | ((u16)(u8)inp(0x42u) << 8));
        first_elapsed = (u16)(0u - timer_count);

        edge_reference = (u8)(inp(status_port) & 0x08u);
        while (((u8)inp(status_port) & 0x08u) == edge_reference) {
        }

        outp(0x43u, 0x80u);
        timer_count = (u8)inp(0x42u);
        timer_count = (u16)(timer_count | ((u16)(u8)inp(0x42u) << 8));
        second_elapsed = (u16)(0u - timer_count);
        second_width = (u16)(second_elapsed - first_elapsed);

        if (((i16)second_width > (i16)first_elapsed) !=
            (first_phase_set != 0u)) {
            break;
        }
        ++retrace_phase;
        break;
    }

    timer_reload_ticks = 3u;
}
