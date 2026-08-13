typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;
typedef signed int i16;
typedef unsigned long u32;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_back_buffer;
extern volatile u8 __based(__segname("GAME_DATA"))
        panorama_transparent_zero;

#define PANORAMA_FRAME_BYTES 64000UL

void far bridge_panorama_frame_unpack(const u8 far *source)
{
    volatile u8 far *output;
    u32 output_remaining;
    u16 count;
    u8 value;
    i8 control;

    output = graphics_back_buffer;
    output_remaining = PANORAMA_FRAME_BYTES;

    if ((panorama_transparent_zero & 1u) != 0) {
        while ((u16)output_remaining != 0) {
            control = (i8)*source++;
            if (control < 0) {
                count = (u16)(-(i16)control + 1);
                output_remaining -= (u32)count;
                value = *source++;
                if (value == 0) {
                    output += count;
                } else {
                    while (count-- != 0) {
                        *output++ = value;
                    }
                }
            } else {
                count = (u16)control + 1u;
                output_remaining -= (u32)count;
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
        while ((u16)output_remaining != 0) {
            control = (i8)*source++;
            if (control < 0) {
                count = (u16)(-(i16)control + 1);
                output_remaining -= (u32)count;
                value = *source++;
                while (count-- != 0) {
                    *output++ = value;
                }
            } else {
                count = (u16)control + 1u;
                output_remaining -= (u32)count;
                while (count-- != 0) {
                    *output++ = *source++;
                }
            }
        }
    }
}

#pragma aux bridge_panorama_frame_unpack parm [ds si]
