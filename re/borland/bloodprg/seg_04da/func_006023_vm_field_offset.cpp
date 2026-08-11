// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006023
// assembly: re/assembly/bloodprg/seg_04da/func_006023_vm_field_offset.asm
// provenance: recursive_graph
// status: translated_vm_field_offset
// reason: mechanical translation of selector/kind bit-scan field-offset table lookup

#include "recovered.hpp"

// label: vm_field_offset

extern "C" void CB_NEAR cb_bloodprg_006023_vm_field_offset(CbMachine* m)
{
    m->push16(m->bx);
    m->ax = (cb_u16)(m->ax << 4);
    cb_u16 bsf_source = m->bx;
    if (bsf_source != 0) {
        cb_u16 bit_index = 0;
        while (((bsf_source >> bit_index) & 1u) == 0) {
            bit_index = (cb_u16)(bit_index + 1);
        }
        m->bx = bit_index;
        m->zf = 0;
    } else {
        m->zf = 1;
    }
    cb_u16 before_add = m->bx;
    m->bx = (cb_u16)(m->bx + m->ax);
    m->set_add16_flags(before_add, m->ax, m->bx);
    cb_set_lo8(m->ax, m->read8(m->gs, (cb_u16)(m->bx + 0x6d60)));
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    m->bx = m->pop16();
    return;
}
