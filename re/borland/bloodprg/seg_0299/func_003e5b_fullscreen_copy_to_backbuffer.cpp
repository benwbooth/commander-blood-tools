// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x003e5b
// assembly: re/assembly/bloodprg/seg_0299/func_003e5b_fullscreen_copy_to_backbuffer.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_fullscreen_dword_copy
// reason: mechanical translation of full-screen REP MOVSD to GS far pointer 0x5229

#include "recovered.hpp"

// label: fullscreen_copy_to_backbuffer

extern "C" void CB_FAR cb_bloodprg_003e5b_fullscreen_copy_to_backbuffer(CbMachine* m)
{
    m->push16(m->cx);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->si);
    m->df = 0;
    m->di = m->read16(m->gs, 0x5229);
    m->es = m->read16(m->gs, 0x522b);
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
