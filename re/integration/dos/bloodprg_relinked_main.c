#include "bloodprg_startup.h"

/*
 * The shipped BLOODPRG keeps SS=GS=DS pinned to the game data segment for
 * its whole lifetime; unrecovered machine code inside this executable still
 * addresses globals through GS-prefixed forms (e.g. the resource
 * materialization loop's `lds dx, gs:[0xa7c]` at 0x283D). Open Watcom never
 * touches GS, so entering those routines from recovered C left GS=0 and the
 * copy loop read its buffer pointer out of the interrupt vector table --
 * the Pterra-entry corruption. Establish the shipped invariant once here,
 * before any game code runs.
 */
extern void bloodprg_set_gs_to_ds(void);
#pragma aux bloodprg_set_gs_to_ds = \
    "mov ax, ds" \
    "mov gs, ax" \
    modify exact [ax];

int main(void)
{
    bloodprg_set_gs_to_ds();
    bloodprg_entry();
    return 0;
}
