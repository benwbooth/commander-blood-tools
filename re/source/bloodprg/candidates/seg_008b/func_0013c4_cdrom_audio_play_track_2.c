#include <dos.h>

#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_startup.h"

void CB_FAR cdrom_audio_play_track_2(void)
{
    volatile bloodprg_mscdex_audio_request CB_FAR *request;
    cb_u32 end_frame;
    cb_u32 end_position;
    cb_u32 start_frame;
    cb_u32 start_position;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present & 1u) == 0u) {
        return;
    }

    request = (volatile bloodprg_mscdex_audio_request CB_FAR *)
            &cdrom_audio_request;
    start_position = cdrom_track_info.start_position;
    end_position = cdrom_disc_info.lead_out_position;

    request->header.length = (cb_u8)sizeof(bloodprg_mscdex_audio_request);
    request->header.command = BLOODPRG_MSCDEX_PLAY_AUDIO;
    request->start_position = start_position;

    start_frame = ((start_position >> 16) & 0xffu) * 4500ul
            + ((start_position >> 8) & 0xffu) * 75ul
            + (start_position & 0xffu) - 150ul;
    end_frame = ((end_position >> 16) & 0xffu) * 4500ul
            + ((end_position >> 8) & 0xffu) * 75ul
            + (end_position & 0xffu) - 150ul;
    request->sector_count = end_frame - start_frame;

    segread(&segments);
    segments.es = FP_SEG(request);
    registers.x.bx = FP_OFF(request);
    registers.x.cx = startup_original_drive;
    registers.x.ax = BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST;
    int86x(BLOODPRG_MSCDEX_INTERRUPT, &registers, &registers, &segments);
}
