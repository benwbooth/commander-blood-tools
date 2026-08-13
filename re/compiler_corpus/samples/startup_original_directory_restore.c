/* Codegen probe for BLOODPRG 0x0027E9. */

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

extern u8 original_drive;
extern char original_directory[32];
extern volatile u8 GAME_DATA write_directory_active;

void far startup_original_directory_restore_probe(void)
{
    unsigned available_drives;

    if ((write_directory_active & 1u) != 0u) {
        _dos_setdrive((unsigned)original_drive + 1u, &available_drives);
        (void)chdir(original_directory);
        write_directory_active = 0u;
    }
}
