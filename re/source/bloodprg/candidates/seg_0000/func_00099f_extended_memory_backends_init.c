#include <dos.h>

#include "../include/bloodprg_ems.h"

static const cb_u8 CB_CODE_DATA ems_device_signature[8] = {
    'E', 'M', 'M', 'X', 'X', 'X', 'X', '0'
};

void CB_FAR extended_memory_backends_init(void)
{
    const volatile cb_u8 CB_FAR *ems_handler;
    const volatile cb_u8 CB_FAR *signature;
    union REGS registers;
    struct SREGS segments;
    cb_u16 handle;
    cb_u16 index;
    int signature_matches;

    ems_handler = (const volatile cb_u8 CB_FAR *)_dos_getvect(0x67u);
    signature = (const volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(ems_handler), 10u);
    signature_matches = 1;
    for (index = 0u; index < 8u; ++index) {
        if (signature[index] != ems_device_signature[index]) {
            signature_matches = 0;
            break;
        }
    }

    if (signature_matches) {
        registers.h.ah = 0x40u;
        int86(0x67, &registers, &registers);
        if (registers.h.ah == 0u) {
            registers.x.bx = 4u;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                small_ems_handle = (cb_i16)registers.x.dx;
            }

            registers.x.bx = 0x10u;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                resource_ems_handle = (cb_i16)registers.x.dx;
            }

            registers.x.bx = 0x10u;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                secondary_ems_handle = (cb_i16)registers.x.dx;
            }

            registers.x.bx = 0x5au;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                snd_bank_ems_handle = (cb_i16)registers.x.dx;
            }

            registers.h.ah = 0x41u;
            int86(0x67, &registers, &registers);
            ems_page_frame_segment = registers.x.bx;
        }
    }

    registers.x.ax = 0x4300u;
    int86(0x2f, &registers, &registers);
    if (registers.h.al != 0x80u) {
        return;
    }

    registers.x.ax = 0x4310u;
    segread(&segments);
    int86x(0x2f, &registers, &registers, &segments);
    xms_driver_entry = (bloodprg_xms_driver_entry)MK_FP(
            segments.es, registers.x.bx);

    if (small_ems_handle == -1 && cb_xms_allocate_kb(0x0040u, &handle)) {
        small_xms_handle = (cb_i16)handle;
    }
    if (resource_ems_handle == -1 && cb_xms_allocate_kb(0x0100u, &handle)) {
        resource_xms_handle = (cb_i16)handle;
    }
    if (secondary_ems_handle == -1 && cb_xms_allocate_kb(0x0100u, &handle)) {
        secondary_xms_handle = (cb_i16)handle;
    }
    if (snd_bank_ems_handle == -1 && cb_xms_allocate_kb(0x05a0u, &handle)) {
        snd_bank_xms_handle = (cb_i16)handle;
    }
}
