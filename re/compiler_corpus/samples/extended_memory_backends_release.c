/* Codegen probe for BLOODPRG 0x000A99. */

#include <dos.h>

typedef signed int i16;
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

extern volatile i16 GAME_DATA resource_xms_handle;
extern volatile i16 GAME_DATA resource_ems_handle;
extern volatile i16 GAME_DATA secondary_xms_handle;
extern volatile i16 GAME_DATA secondary_ems_handle;
extern volatile i16 GAME_DATA snd_bank_xms_handle;
extern volatile i16 GAME_DATA snd_bank_ems_handle;
extern volatile i16 GAME_DATA small_xms_handle;
extern volatile i16 GAME_DATA small_ems_handle;
extern void NEAR xms_release(u16 handle);

void FAR extended_memory_backends_release_probe(void)
{
    union REGS registers;

    if (small_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (u16)small_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (resource_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (u16)resource_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (secondary_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (u16)secondary_ems_handle;
        int86(0x67, &registers, &registers);
    }
    if (snd_bank_ems_handle != -1) {
        registers.h.ah = 0x45u;
        registers.x.dx = (u16)snd_bank_ems_handle;
        int86(0x67, &registers, &registers);
    }

    if (small_xms_handle != -1) {
        xms_release((u16)small_xms_handle);
    }
    if (resource_xms_handle != -1) {
        xms_release((u16)resource_xms_handle);
    }
    if (secondary_xms_handle != -1) {
        xms_release((u16)secondary_xms_handle);
    }
    if (snd_bank_xms_handle != -1) {
        xms_release((u16)snd_bank_xms_handle);
    }
}
