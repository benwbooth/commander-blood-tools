// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x003e46
// assembly: re/assembly/bloodprg/seg_0299/func_003e46_full_screen_blit.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_fullscreen_dword_copy
// reason: mechanical translation of full-screen REP MOVSD to GS far pointer 0x5221

#include "recovered.hpp"

// label: full_screen_blit

extern "C" void CB_FAR cb_bloodprg_003e46_full_screen_blit(CbMachine* m)
{
    m->push16(m->cx);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->si);
    m->df = 0;
    m->di = m->read16(m->gs, 0x5221);
    m->es = m->read16(m->gs, 0x5223);
    m->cx = 0x3e80;
    while (m->cx != 0) {
        cb_u32 value = m->read32(m->ds, m->si);
        m->write32(m->es, m->di, value);
        cb_advance_u16(m->si, 4, m->df);
        cb_advance_u16(m->di, 4, m->df);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->si = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->cx = m->pop16();
    return;
}
