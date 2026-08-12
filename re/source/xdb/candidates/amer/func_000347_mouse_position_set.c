#include "../include/xdb_mouse.h"

void XDB_NEAR xdb_amer_mouse_position_set(xdb_u16 x, xdb_u16 y)
{
    xdb_alien_mouse_x = x;
    xdb_alien_mouse_y = y;
    xdb_mouse_driver_set_position();
}
