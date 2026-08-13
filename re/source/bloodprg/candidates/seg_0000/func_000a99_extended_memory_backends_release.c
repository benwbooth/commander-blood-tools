#include <dos.h>

#include "../include/bloodprg_ems.h"

void CB_FAR extended_memory_backends_release(void)
{
    union REGS registers;

    if (small_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (cb_u16)small_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (resource_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (cb_u16)resource_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (secondary_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (cb_u16)secondary_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (snd_bank_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (cb_u16)snd_bank_ems_handle;
        int86(0x67, &registers, &registers);
    }

    if (small_xms_handle != -1) {
        cb_xms_release((cb_u16)small_xms_handle);
    }
    if (resource_xms_handle != -1) {
        cb_xms_release((cb_u16)resource_xms_handle);
    }
    if (secondary_xms_handle != -1) {
        cb_xms_release((cb_u16)secondary_xms_handle);
    }
    if (snd_bank_xms_handle != -1) {
        cb_xms_release((cb_u16)snd_bank_xms_handle);
    }
}
