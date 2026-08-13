#include <dos.h>

#include "../include/bloodprg_input.h"

void CB_FAR poll_mouse(void)
{
    union REGS registers;

    registers.x.ax = 3u;
    int86(0x33, &registers, &registers);

    mouse_x = (cb_i16)registers.x.cx;
    mouse_y = (cb_i16)registers.x.dx;
    mouse_button_state = registers.x.bx;

    if (mouse_last_x != (cb_i16)registers.x.cx ||
            mouse_last_y != (cb_i16)registers.x.dx) {
        mouse_last_x = (cb_i16)registers.x.cx;
        mouse_last_y = (cb_i16)registers.x.dx;
        mouse_motion_idle_counter = 0u;
    }
}
