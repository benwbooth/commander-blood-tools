// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x005ff6
// assembly: re/assembly/bloodprg/seg_04da/func_005ff6_vm_special_slot_insert.asm
// provenance: recursive_graph
// status: translated_vm_special_slot_insert
// reason: mechanical translation of 16-word sentinel-list insert/present probe

#include "recovered.hpp"

// label: vm_special_slot_insert

extern "C" void CB_NEAR cb_bloodprg_005ff6_vm_special_slot_insert(CbMachine* m)
{
    m->push16(m->cx);
    m->push16(m->bp);
    m->bp = 0x6d3e;
    m->cx = 0x0010;
    int found = 0;
    while (m->cx != 0) {
        cb_u16 slot = m->read16(m->ds, m->bp);
        cb_u16 cmp_result = (cb_u16)(m->ax - slot);
        m->set_sub16_flags(m->ax, slot, cmp_result);
        if (cmp_result == 0) {
            found = 1;
            break;
        }
        cb_u16 before_add = m->bp;
        m->bp = (cb_u16)(m->bp + 2);
        m->set_add16_flags(before_add, 2, m->bp);
        m->cx = (cb_u16)(m->cx - 1);
    }
    if (found == 0) {
        m->bp = 0x6d3e;
        m->cx = 0x0010;
        while (m->cx != 0) {
            cb_u16 slot = m->read16(m->ds, m->bp);
            cb_u16 cmp_result = (cb_u16)(slot - 0);
            m->set_sub16_flags(slot, 0, cmp_result);
            if (cmp_result == 0) {
                m->write16(m->ds, m->bp, m->ax);
                found = 1;
                break;
            }
            cb_u16 before_add = m->bp;
            m->bp = (cb_u16)(m->bp + 2);
            m->set_add16_flags(before_add, 2, m->bp);
            m->cx = (cb_u16)(m->cx - 1);
        }
    }
    if (found != 0) {
        m->cf = 1;
    } else {
        m->cf = 0;
    }
    m->bp = m->pop16();
    m->cx = m->pop16();
    return;
}
