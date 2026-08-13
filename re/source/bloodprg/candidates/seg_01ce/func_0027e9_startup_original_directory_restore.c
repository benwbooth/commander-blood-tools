#include <dos.h>
#if defined(__TURBOC__) || defined(__BORLANDC__)
#include <dir.h>
#else
#include <direct.h>
#endif

#include "../include/bloodprg_startup.h"

void CB_FAR startup_original_directory_restore(void)
{
    unsigned available_drives;

    if ((startup_write_directory_active & 1u) != 0u) {
        _dos_setdrive((unsigned)startup_original_drive + 1u, &available_drives);
        (void)chdir(startup_original_directory);
        startup_write_directory_active = 0u;
    }
}
