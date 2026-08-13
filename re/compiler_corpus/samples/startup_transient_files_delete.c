/* Codegen probe for BLOODPRG 0x00147F. */

#include <stdio.h>

typedef unsigned int u16;

extern char transient_paths[4][16];

void near startup_transient_files_delete_probe(void)
{
    u16 index;

    for (index = 0u; index < 4u; ++index) {
        if (transient_paths[index][0] != 'x') {
            (void)remove(transient_paths[index]);
        }
    }
}
