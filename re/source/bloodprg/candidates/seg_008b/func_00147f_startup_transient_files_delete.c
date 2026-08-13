#include <stdio.h>

#include "../include/bloodprg_startup.h"

void CB_NEAR startup_transient_files_delete(void)
{
    cb_u16 index;

    for (index = 0u; index < 4u; ++index) {
        if (startup_transient_paths[index][0] != 'x') {
            (void)remove(startup_transient_paths[index]);
        }
    }
}
