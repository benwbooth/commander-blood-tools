/* Codegen probe for BLOODPRG 0x001397. */

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

typedef struct audio_request_probe {
    request_header_probe header;
    u8 address_mode;
    u32 start_position;
    u32 sector_count;
} audio_request_probe;
#pragma pack()

extern volatile u8 cdrom_present_probe;
extern volatile u8 startup_original_drive_probe;
extern volatile audio_request_probe GAME_DATA cdrom_audio_request_probe;

void FAR cdrom_audio_stop_probe(void)
{
    volatile audio_request_probe FAR *request;
    union REGS registers;
    struct SREGS segments;

    if ((cdrom_present_probe & 1u) == 0u) {
        return;
    }

    request = (volatile audio_request_probe FAR *)&cdrom_audio_request_probe;
    request->header.length = 0x0du;
    request->header.command = 0x85u;

    segread(&segments);
    segments.es = FP_SEG(request);
    registers.x.bx = FP_OFF(request);
    registers.x.cx = startup_original_drive_probe;
    registers.x.ax = 0x1510u;
    int86x(0x2f, &registers, &registers, &segments);
}
