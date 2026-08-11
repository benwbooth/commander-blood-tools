// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x001d94
// assembly: re/assembly/bloodprg/seg_008b/func_001d94_vm_context_pointer_setup.asm
// provenance: recursive_graph
// status: translated_vm_context_pointer_setup
// reason: mechanical translation of VM context pointer setup and COD work-buffer list build

#include "recovered.hpp"

// label: vm_context_pointer_setup

extern "C" void CB_NEAR cb_bloodprg_001d94_vm_context_pointer_setup(CbMachine* m)
{
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->push16(m->fs);
    m->push16(m->cx);
    m->push16(m->bp);
    m->di = m->read16(m->gs, 0x0abc);
    m->es = m->read16(m->gs, 0x0abe);
    m->si = m->read16(m->gs, 0x671c);
    m->ds = m->read16(m->gs, 0x671e);
    m->bp = m->read16(m->gs, 0x672c);
    m->fs = m->read16(m->gs, 0x672e);
    m->cx = 0;
    m->set_logic16_flags(m->cx);
    for (;;) {
        cb_u16 marker = m->read16(m->fs, (cb_u16)(m->bp + 0x10));
        cb_u16 cmp_marker = (cb_u16)(marker - 0xffff);
        m->set_sub16_flags(marker, 0xffff, cmp_marker);
        if (cmp_marker == 0) {
            break;
        }
        cb_u16 state = m->read16(m->fs, (cb_u16)(m->bp + 0x12));
        cb_u16 cmp_state = (cb_u16)(state - 2);
        m->set_sub16_flags(state, 2, cmp_state);
        if (cmp_state == 0) {
            m->ax = m->read16(m->fs, (cb_u16)(m->bp + 0x10));
            m->write16(m->es, m->di, m->ax);
            cb_advance_u16(m->di, 2, m->df);
            m->si = m->ax;
            cb_set_lo8(m->ax, m->read8(m->ds, m->si));
            cb_advance_u16(m->si, 1, m->df);
            m->write8(m->es, m->di, cb_lo8(m->ax));
            cb_advance_u16(m->di, 1, m->df);
            cb_u16 before_add_cx = m->cx;
            m->cx = (cb_u16)(m->cx + 3);
            m->set_add16_flags(before_add_cx, 3, m->cx);
        }
        cb_u16 before_add_bp = m->bp;
        m->bp = (cb_u16)(m->bp + 0x14);
        m->set_add16_flags(before_add_bp, 0x14, m->bp);
    }
    m->ax = m->cx;
    m->bp = m->pop16();
    m->cx = m->pop16();
    m->fs = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    return;
}
