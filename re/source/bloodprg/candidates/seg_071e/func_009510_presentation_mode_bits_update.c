#include "../include/bloodprg_vm.h"

void CB_NEAR presentation_mode_bits_update(void)
{
    cb_u16 flags;
    cb_u16 mode;
    cb_i16 frame;

    flags = (cb_u16)(vm_ui_state.word & 0xff0fu);
    if ((flags & 2u) == 0) {
        mode = 1u;
        frame = vm_bridge_view_frame;
        if (frame > 0x16 && frame <= 0x9d) {
            mode = (cb_u16)(mode << 1);
            if (frame > 0x43) {
                mode = (cb_u16)(mode << 1);
                if (frame > 0x70) {
                    mode = (cb_u16)(mode << 1);
                }
            }
        }
        flags = (cb_u16)(flags | (mode << 4));
    }

    vm_ui_state.word = flags;
}
