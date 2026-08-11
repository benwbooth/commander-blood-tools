// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x005afd
// assembly: re/assembly/bloodprg/seg_04da/func_005afd_vm_op_a3_collect.asm
// provenance: recursive_graph
// status: translated_vm_op_a3_collect
// reason: mechanical translation of VM opcode 0xa3 word-list collection into GS:0x67f8

#include "recovered.hpp"

// label: vm_op_a3_collect

extern "C" void CB_NEAR cb_bloodprg_005afd_vm_op_a3_collect(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->si = m->read16(m->gs, 0x6720);
    m->ds = m->read16(m->gs, 0x6722);
    m->si = m->read16(m->gs, 0x6772);
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_u8 al = cb_lo8(m->ax);
    cb_u8 cmp_result = (cb_u8)(al - 0xa3);
    m->set_sub8_flags(al, 0xa3, cmp_result);
    if (cmp_result == 0) {
        cb_u16 before_inc = m->si;
        m->si = (cb_u16)(m->si + 1);
        m->set_inc16_flags(before_inc, m->si);
        m->ax = m->gs;
        m->es = m->ax;
        m->di = 0x67f8;
        for (;;) {
            m->ax = m->read16(m->ds, m->si);
            cb_advance_u16(m->si, 2, m->df);
            m->set_logic16_flags(m->ax);
            if (m->ax == 0) {
                break;
            }
            m->write16(m->es, m->di, m->ax);
            cb_advance_u16(m->di, 2, m->df);
        }
        m->ax = m->read16(m->gs, 0x6770);
        m->set_logic16_flags(m->ax);
        if (m->ax != 0) {
            m->write16(m->es, m->di, m->ax);
            cb_advance_u16(m->di, 2, m->df);
            m->ax = 0;
            m->set_logic16_flags(m->ax);
            m->write16(m->gs, 0x6770, m->ax);
        }
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
    }
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->ax = m->pop16();
    return;
}
