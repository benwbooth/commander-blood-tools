/* Codegen probe for BLOODPRG 0x00099F. */

#include <dos.h>

typedef signed int i16;
typedef unsigned char u8;
typedef unsigned int u16;

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
#define GAME_DATA
#endif

typedef void (FAR *xms_driver_entry_type)(void);

extern xms_driver_entry_type GAME_DATA xms_driver_entry;
extern volatile i16 GAME_DATA resource_xms_handle;
extern volatile i16 GAME_DATA resource_ems_handle;
extern volatile i16 GAME_DATA secondary_xms_handle;
extern volatile i16 GAME_DATA secondary_ems_handle;
extern volatile i16 GAME_DATA archive_xms_handle;
extern volatile i16 GAME_DATA archive_ems_handle;
extern volatile i16 GAME_DATA small_xms_handle;
extern volatile i16 GAME_DATA small_ems_handle;
extern volatile u16 GAME_DATA ems_page_frame_segment;
extern int NEAR xms_allocate_kb(u16 kilobytes, u16 *handle);

static const u8 ems_device_signature[8] = {
    'E', 'M', 'M', 'X', 'X', 'X', 'X', '0'
};

void FAR extended_memory_backends_init_probe(void)
{
    const volatile u8 FAR *ems_handler;
    const volatile u8 FAR *signature;
    union REGS registers;
    struct SREGS segments;
    u16 handle;
    u16 index;
    int signature_matches;

    ems_handler = (const volatile u8 FAR *)_dos_getvect(0x67u);
    signature = (const volatile u8 FAR *)MK_FP(FP_SEG(ems_handler), 10u);
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
                small_ems_handle = (i16)registers.x.dx;
            }

            registers.x.bx = 0x10u;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                resource_ems_handle = (i16)registers.x.dx;
            }

            registers.x.bx = 0x10u;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                secondary_ems_handle = (i16)registers.x.dx;
            }

            registers.x.bx = 0x5au;
            registers.h.ah = 0x43u;
            int86(0x67, &registers, &registers);
            if (registers.h.ah == 0u) {
                archive_ems_handle = (i16)registers.x.dx;
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
    xms_driver_entry = (xms_driver_entry_type)MK_FP(
            segments.es, registers.x.bx);

    if (small_ems_handle == -1 && xms_allocate_kb(0x0040u, &handle)) {
        small_xms_handle = (i16)handle;
    }
    if (resource_ems_handle == -1 && xms_allocate_kb(0x0100u, &handle)) {
        resource_xms_handle = (i16)handle;
    }
    if (secondary_ems_handle == -1 && xms_allocate_kb(0x0100u, &handle)) {
        secondary_xms_handle = (i16)handle;
    }
    if (archive_ems_handle == -1 && xms_allocate_kb(0x05a0u, &handle)) {
        archive_xms_handle = (i16)handle;
    }
}
