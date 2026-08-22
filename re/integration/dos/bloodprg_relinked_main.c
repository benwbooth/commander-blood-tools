#include "bloodprg_resource.h"
#include "bloodprg_startup.h"
#include <i86.h>

/*
 * The shipped BLOODPRG entrypoint establishes GS=DS and gives FS its resource
 * table segment. Recovered C uses compiler-relocated based/far pointers and
 * has no FS/GS-prefixed data accesses, but loaded foreign callbacks may retain
 * the original convention. Install it once at that integration boundary.
 * GAME_DATA and FS_DATA are paragraph-aligned and begin at offset zero, as
 * enforced by the package layout audit.
 */
extern void bloodprg_install_game_segments(unsigned fs_segment);
#pragma aux bloodprg_install_game_segments = \
    "mov dx, ds" \
    "mov gs, dx" \
    "mov fs, ax" \
    parm [ax] \
    modify exact [dx];

int main(void)
{
    bloodprg_install_game_segments(FP_SEG(fs_resource_handle_table));
    bloodprg_entry();
    return 0;
}
