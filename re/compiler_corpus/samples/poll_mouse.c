/* Codegen probe for BLOODPRG 0x000D0E. */

#include <dos.h>

typedef signed int i16;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

extern volatile i16 mouse_x;
extern volatile i16 mouse_y;
extern volatile u16 mouse_button_state;
extern volatile i16 mouse_last_x;
extern volatile i16 mouse_last_y;
extern volatile u16 mouse_motion_idle_counter;

void FAR poll_mouse_probe(void)
{
    union REGS registers;

    registers.x.ax = 3u;
    int86(0x33, &registers, &registers);

    mouse_x = (i16)registers.x.cx;
    mouse_y = (i16)registers.x.dx;
    mouse_button_state = registers.x.bx;

    if (mouse_last_x != (i16)registers.x.cx ||
            mouse_last_y != (i16)registers.x.dx) {
        mouse_last_x = (i16)registers.x.cx;
        mouse_last_y = (i16)registers.x.dx;
        mouse_motion_idle_counter = 0u;
    }
}
