#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"

#define NAV_WIPE_CENTER_X 160u
#define NAV_WIPE_CENTER_Y 110u
#define NAV_WIPE_SPAN_END 0xffffu

void CB_NEAR nav_center_wipe_span_table_build(
        const volatile bloodprg_nav_wipe_point CB_NEAR *endpoint)
{
    volatile bloodprg_nav_wipe_span CB_FAR *output;
    cb_u16 start_x;
    cb_u16 start_y;
    cb_u16 end_x;
    cb_u16 end_y;
    cb_u16 horizontal_delta;
    cb_u16 vertical_delta;
    cb_u16 doubled_horizontal;
    cb_u16 doubled_vertical;
    cb_u16 error;
    cb_u16 iterations;
    cb_i16 x_step;

    start_x = (cb_u16)endpoint->x;
    start_y = (cb_u16)endpoint->y;
    end_x = NAV_WIPE_CENTER_X;
    end_y = NAV_WIPE_CENTER_Y;
    if ((cb_i16)start_y >= (cb_i16)NAV_WIPE_CENTER_Y) {
        end_x = start_x;
        end_y = start_y;
        start_x = NAV_WIPE_CENTER_X;
        start_y = NAV_WIPE_CENTER_Y;
    }

    horizontal_delta = (cb_u16)(end_x - start_x);
    vertical_delta = (cb_u16)(end_y - start_y);
    x_step = 1;
    if ((cb_i16)horizontal_delta < 0) {
        horizontal_delta = (cb_u16)(0u - horizontal_delta);
        x_step = -1;
    }

    output = (volatile bloodprg_nav_wipe_span CB_FAR *)
            graphics_display_buffer_ds;
    doubled_horizontal = (cb_u16)(horizontal_delta * 2u);
    doubled_vertical = (cb_u16)(vertical_delta * 2u);

    if ((cb_i16)vertical_delta >= (cb_i16)horizontal_delta) {
        error = (cb_u16)(doubled_horizontal - vertical_delta);
        iterations = vertical_delta;
        do {
            output->left = start_x;
            output->width = (cb_u16)(
                    (cb_u16)(NAV_WIPE_CENTER_X - start_x) * 2u);
            ++output;
            if ((cb_i16)error >= 0) {
                start_x = (cb_u16)(start_x + (cb_u16)x_step);
                error = (cb_u16)(error - doubled_vertical);
            }
            error = (cb_u16)(error + doubled_horizontal);
        } while (--iterations != 0u);
    } else {
        error = (cb_u16)(doubled_vertical - horizontal_delta);
        iterations = horizontal_delta;
        do {
            start_x = (cb_u16)(start_x + (cb_u16)x_step);
            if ((cb_i16)error >= 0) {
                output->left = start_x;
                output->width = (cb_u16)(
                        (cb_u16)(NAV_WIPE_CENTER_X - start_x) * 2u);
                ++output;
                error = (cb_u16)(error - doubled_horizontal);
            }
            error = (cb_u16)(error + doubled_vertical);
        } while (--iterations != 0u);
    }

    output->left = NAV_WIPE_SPAN_END;
    output->width = NAV_WIPE_SPAN_END;
}
