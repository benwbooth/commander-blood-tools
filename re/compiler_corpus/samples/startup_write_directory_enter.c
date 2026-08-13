/* Codegen probe for BLOODPRG 0x0027C3. */

#include <dos.h>
#if defined(__TURBOC__) || defined(__BORLANDC__)
#include <dir.h>
#else
#include <direct.h>
#endif

typedef unsigned char u8;

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA far
#endif

extern u8 write_drive;
extern char write_directory[32];
extern volatile u8 GAME_DATA write_directory_active;

void far startup_write_directory_enter_probe(void)
{
    unsigned available_drives;

    if ((write_directory_active & 1u) == 0u) {
        _dos_setdrive((unsigned)write_drive + 1u, &available_drives);
        (void)chdir(write_directory);
        write_directory_active = 1u;
    }
}
