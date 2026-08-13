#include <dos.h>

#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_startup.h"

void CB_NEAR cdrom_audio_prepare(void)
{
    volatile bloodprg_mscdex_ioctl_request CB_FAR *request;
    volatile void CB_FAR *transfer;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present & 1u) == 0u) {
        return;
    }

    request = (volatile bloodprg_mscdex_ioctl_request CB_FAR *)
            &cdrom_ioctl_request;
    transfer = (volatile void CB_FAR *)&cdrom_disc_info;
    request->header.command = BLOODPRG_MSCDEX_IOCTL_INPUT;
    request->transfer_offset = FP_OFF(transfer);
    request->transfer_segment = FP_SEG(transfer);

    segread(&segments);
    segments.es = FP_SEG(request);
    registers.x.bx = FP_OFF(request);
    registers.x.cx = startup_original_drive;
    registers.x.ax = BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST;
    int86x(BLOODPRG_MSCDEX_INTERRUPT, &registers, &registers, &segments);

    transfer = (volatile void CB_FAR *)&cdrom_track_info;
    request->transfer_offset = FP_OFF(transfer);
    cdrom_track_info.track_number = 2u;
    registers.x.ax = BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST;
    int86x(BLOODPRG_MSCDEX_INTERRUPT, &registers, &registers, &segments);

    transfer = (volatile void CB_FAR *)&cdrom_channel_control;
    request->header.command = BLOODPRG_MSCDEX_IOCTL_OUTPUT;
    request->transfer_offset = FP_OFF(transfer);
    request->transfer_count = (cb_u16)sizeof(cdrom_channel_control);
    registers.x.ax = BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST;
    int86x(BLOODPRG_MSCDEX_INTERRUPT, &registers, &registers, &segments);
}
