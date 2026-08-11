// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x008c96
// assembly: re/assembly/bloodprg/seg_071e/func_008c96_vm_segment_call_wrapper.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_vm_segment_call_wrapper
// reason: mechanical translation of VM far-call wrapper and GS dword-copy postamble

#include "recovered.hpp"

// label: vm_segment_call_wrapper

extern "C" void CB_FAR cb_bloodprg_008c96_vm_segment_call_wrapper(CbMachine* m)
{
    m->push16(m->bp);
    m->push16(m->ax);
    m->push16(m->ds);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->si);
    m->push16(m->cx);
    m->call_far(0x04da, 0x1c53);
    m->ax = m->gs;
    m->ds = m->ax;
    m->es = m->ax;
    m->si = 0x53d1;
    m->di = 0x5cd8;
    m->cx = 0x0030;
    while (m->cx != 0) {
        cb_u32 value = m->read32(m->ds, m->si);
        m->write32(m->es, m->di, value);
        cb_advance_u16(m->si, 4, m->df);
        cb_advance_u16(m->di, 4, m->df);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->write16(m->ds, 0x2f65, 0x2710);
    m->write16(m->ds, 0x2f67, 0x2ee0);
    m->write16(m->ds, 0x2f69, 0);
    m->cx = m->pop16();
    m->si = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->ds = m->pop16();
    m->ax = m->pop16();
    m->bp = m->pop16();
    return;
}
