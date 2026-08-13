/* Codegen probe for BLOODPRG 0x0013C4. */

#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
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

typedef struct disc_info_probe {
    u8 function;
    u8 first_track;
    u8 last_track;
    u32 lead_out_position;
} disc_info_probe;

typedef struct track_info_probe {
    u8 function;
    u8 track_number;
    u32 start_position;
    u8 control;
} track_info_probe;

typedef struct audio_request_probe {
    request_header_probe header;
    u8 address_mode;
    u32 start_position;
    u32 sector_count;
} audio_request_probe;
#pragma pack()

extern volatile u8 cdrom_present_probe;
extern volatile u8 startup_original_drive_probe;
extern volatile disc_info_probe GAME_DATA cdrom_disc_info_probe;
extern volatile track_info_probe GAME_DATA cdrom_track_info_probe;
extern volatile audio_request_probe GAME_DATA cdrom_audio_request_probe;

void FAR cdrom_audio_play_track_2_probe(void)
{
    volatile audio_request_probe FAR *request;
    u32 end_frame;
    u32 end_position;
    u32 start_frame;
    u32 start_position;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present_probe & 1u) == 0u) {
        return;
    }

    request = (volatile audio_request_probe FAR *)&cdrom_audio_request_probe;
    start_position = cdrom_track_info_probe.start_position;
    end_position = cdrom_disc_info_probe.lead_out_position;

    request->header.length = 0x16u;
    request->header.command = 0x84u;
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
    registers.x.cx = startup_original_drive_probe;
    registers.x.ax = 0x1510u;
    int86x(0x2f, &registers, &registers, &segments);
}
