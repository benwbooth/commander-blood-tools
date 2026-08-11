// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000b32
// assembly: re/assembly/bloodprg/seg_0000/func_000b32_detect_cdrom.asm
// provenance: recursive_graph
// status: translated_detect_cdrom
// reason: mechanical translation of MSCDEX int 2fh probe and GS flag store

#include "recovered.hpp"

// label: detect_cdrom

extern "C" void CB_NEAR cb_bloodprg_000b32_detect_cdrom(CbMachine* m)
{
    m->ax = 0x1500;
    m->bx = 0;
    m->set_logic16_flags(m->bx);
    m->interrupt(0x2f);
    m->set_logic16_flags(m->bx);
    m->write8(m->gs, 0x0ae6, m->bx != 0 ? 1 : 0);
    return;
}
