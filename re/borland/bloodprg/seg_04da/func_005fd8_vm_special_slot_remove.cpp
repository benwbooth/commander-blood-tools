// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x005fd8
// assembly: re/assembly/bloodprg/seg_04da/func_005fd8_vm_special_slot_remove.asm
// provenance: recursive_graph
// status: translated_vm_special_slot_remove
// reason: mechanical translation of 16-word sentinel-list remove preserving loop flags

#include "recovered.hpp"

// label: vm_special_slot_remove

extern "C" void CB_NEAR cb_bloodprg_005fd8_vm_special_slot_remove(CbMachine* m)
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
            m->write16(m->ds, m->bp, 0);
            m->cf = 1;
            found = 1;
            break;
        }
        cb_u16 before_add = m->bp;
        m->bp = (cb_u16)(m->bp + 2);
        m->set_add16_flags(before_add, 2, m->bp);
        m->cx = (cb_u16)(m->cx - 1);
    }
    if (found == 0) {
        m->cf = 0;
    }
    m->bp = m->pop16();
    m->cx = m->pop16();
    return;
}
