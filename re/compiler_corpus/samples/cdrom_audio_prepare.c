/* Codegen probe for BLOODPRG 0x001344. */

#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

#pragma pack(1)
typedef struct request_header_probe {
    u8 length;
    u8 subunit;
    u8 command;
    u16 status;
    u8 reserved[8];
} request_header_probe;

typedef struct ioctl_request_probe {
    request_header_probe header;
    u8 media_descriptor;
    u16 transfer_offset;
    u16 transfer_segment;
    u16 transfer_count;
    u8 untouched_tail[6];
} ioctl_request_probe;

typedef struct disc_info_probe {
    u8 function;
    u8 first_track;
    u8 last_track;
    u32 lead_out_position;
} disc_info_probe;

typedef struct channel_control_probe {
    u8 function;
    u8 input_channel_0;
    u8 volume_0;
    u8 input_channel_1;
    u8 volume_1;
    u8 input_channel_2;
    u8 volume_2;
    u8 input_channel_3;
    u8 volume_3;
} channel_control_probe;

typedef struct track_info_probe {
    u8 function;
    u8 track_number;
    u32 start_position;
    u8 control;
} track_info_probe;
#pragma pack()

extern volatile u8 cdrom_present_probe;
extern volatile u8 startup_original_drive_probe;
extern volatile ioctl_request_probe GAME_DATA cdrom_ioctl_request_probe;
extern volatile disc_info_probe GAME_DATA cdrom_disc_info_probe;
extern volatile channel_control_probe GAME_DATA cdrom_channel_control_probe;
extern volatile track_info_probe GAME_DATA cdrom_track_info_probe;

void NEAR cdrom_audio_prepare_probe(void)
{
    volatile ioctl_request_probe FAR *request;
    volatile void FAR *transfer;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present_probe & 1u) == 0u) {
        return;
    }

    request = (volatile ioctl_request_probe FAR *)&cdrom_ioctl_request_probe;
    transfer = (volatile void FAR *)&cdrom_disc_info_probe;
    request->header.command = 0x03u;
    request->transfer_offset = FP_OFF(transfer);
    request->transfer_segment = FP_SEG(transfer);

    segread(&segments);
    segments.es = FP_SEG(request);
    registers.x.bx = FP_OFF(request);
    registers.x.cx = startup_original_drive_probe;
    registers.x.ax = 0x1510u;
    int86x(0x2f, &registers, &registers, &segments);

    transfer = (volatile void FAR *)&cdrom_track_info_probe;
    request->transfer_offset = FP_OFF(transfer);
    cdrom_track_info_probe.track_number = 2u;
    registers.x.ax = 0x1510u;
    int86x(0x2f, &registers, &registers, &segments);

    transfer = (volatile void FAR *)&cdrom_channel_control_probe;
    request->header.command = 0x0cu;
    request->transfer_offset = FP_OFF(transfer);
    request->transfer_count = 9u;
    registers.x.ax = 0x1510u;
    int86x(0x2f, &registers, &registers, &segments);
}
