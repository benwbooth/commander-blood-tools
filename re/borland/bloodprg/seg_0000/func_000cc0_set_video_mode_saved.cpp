// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000cc0
// assembly: re/assembly/bloodprg/seg_0000/func_000cc0_set_video_mode_saved.asm
// provenance: recursive_graph
// status: translated_set_video_mode_saved
// reason: mechanical translation of BIOS int 10h video mode restore preserving AX

#include "recovered.hpp"

// label: set_video_mode_saved

extern "C" void CB_FAR cb_bloodprg_000cc0_set_video_mode_saved(CbMachine* m)
{
    m->push16(m->ax);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    cb_set_lo8(m->ax, m->read8(m->gs, 0x5232));
    m->interrupt(0x10);
    m->ax = m->pop16();
    return;
}
