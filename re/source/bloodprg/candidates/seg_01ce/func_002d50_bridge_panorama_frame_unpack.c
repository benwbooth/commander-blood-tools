#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PANORAMA_FRAME_BYTES 64000UL

void CB_FAR bridge_panorama_frame_unpack(
        const cb_u8 CB_FAR *source)
{
    volatile cb_u8 CB_FAR *output;
    cb_u32 output_remaining;
    cb_u16 count;
    cb_u8 value;
    cb_i8 control;

    output = graphics_back_buffer;
    output_remaining = BLOODPRG_PANORAMA_FRAME_BYTES;

    if ((pbm_transparent_zero & 1u) != 0) {
        while ((cb_u16)output_remaining != 0) {
            control = (cb_i8)*source++;
            if (control < 0) {
                count = (cb_u16)(-(cb_i16)control + 1);
                output_remaining -= (cb_u32)count;
                value = *source++;
                if (value == 0) {
                    output += count;
                } else {
                    while (count-- != 0) {
                        *output++ = value;
                    }
                }
            } else {
                count = (cb_u16)control + 1u;
                output_remaining -= (cb_u32)count;
                while (count-- != 0) {
                    value = *source++;
                    if (value != 0) {
                        *output = value;
                    }
                    ++output;
                }
            }
        }
    } else {
        while ((cb_u16)output_remaining != 0) {
            control = (cb_i8)*source++;
            if (control < 0) {
                count = (cb_u16)(-(cb_i16)control + 1);
                output_remaining -= (cb_u32)count;
                value = *source++;
                while (count-- != 0) {
                    *output++ = value;
                }
            } else {
                count = (cb_u16)control + 1u;
                output_remaining -= (cb_u32)count;
                while (count-- != 0) {
                    *output++ = *source++;
                }
            }
        }
    }
}
