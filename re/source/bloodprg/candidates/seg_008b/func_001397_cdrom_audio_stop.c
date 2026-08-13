#include <dos.h>

#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_startup.h"

void CB_FAR cdrom_audio_stop(void)
{
    volatile bloodprg_mscdex_audio_request CB_FAR *request;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present & 1u) == 0u) {
        return;
    }

    request = (volatile bloodprg_mscdex_audio_request CB_FAR *)
            &cdrom_audio_request;
    request->header.length = (cb_u8)sizeof(bloodprg_mscdex_request_header);
    request->header.command = BLOODPRG_MSCDEX_STOP_AUDIO;

    segread(&segments);
    segments.es = FP_SEG(request);
    registers.x.bx = FP_OFF(request);
    registers.x.cx = startup_original_drive;
    registers.x.ax = BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST;
    int86x(BLOODPRG_MSCDEX_INTERRUPT, &registers, &registers, &segments);
}
