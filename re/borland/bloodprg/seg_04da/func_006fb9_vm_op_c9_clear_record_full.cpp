// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006fb9
// assembly: re/assembly/bloodprg/seg_04da/func_006fb9_vm_op_c9_clear_record_full.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_c9_clear_record_full
// reason: mechanical translation of VM opcode 0xc9 three-word record clear and reciprocal C4 teardown

#include "recovered.hpp"

// label: vm_op_c9_clear_record_full

extern "C" void CB_NEAR cb_bloodprg_006fb9_vm_op_c9_clear_record_full(CbMachine* m)
{
    m->push16(m->di);
    m->di = m->read16(m->gs, 0x6724);
    m->es = m->read16(m->gs, 0x6726);
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->di = m->ax;
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    m->cx = m->read16(m->es, m->di);
    m->write16(m->es, m->di, m->ax);
    cb_advance_u16(m->di, 2, m->df);
    m->bx = m->read16(m->es, m->di);
    m->write16(m->es, m->di, m->ax);
    cb_advance_u16(m->di, 2, m->df);
    m->write16(m->es, m->di, m->ax);
    cb_advance_u16(m->di, 2, m->df);
    cb_u16 cmp_result = (cb_u16)(m->cx - 0x00c4);
    m->set_sub16_flags(m->cx, 0x00c4, cmp_result);
    if (cmp_result == 0) {
        m->push16(m->bx);
        m->bx = m->read16(m->es, m->bx);
        m->ax = 0x0013;
        m->call_near(0x6023);
        m->di = m->pop16();
        cb_u16 before_add = m->di;
        m->di = (cb_u16)(m->di + m->ax);
        m->set_add16_flags(before_add, m->ax, m->di);
        m->ax = 0;
        m->set_logic16_flags(m->ax);
        m->write8(m->gs, 0x252a, 0);
        m->write8(m->gs, 0x2531, 6);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
    }
    m->di = m->pop16();
    return;
}
