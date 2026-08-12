#include "../include/xdb_mouse.h"

void XDB_NEAR xdb_croolis_mouse_bounds_set(xdb_u16 max_x, xdb_u16 max_y)
{
    xdb_mouse_driver_set_vertical_bounds(0, max_y);
    xdb_mouse_driver_set_horizontal_bounds(0, max_x);
}
