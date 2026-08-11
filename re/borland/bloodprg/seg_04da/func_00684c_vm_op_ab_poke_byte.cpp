// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00684c
// assembly: re/assembly/bloodprg/seg_04da/func_00684c_vm_op_ab_poke_byte.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_ab_poke_byte
// reason: mechanical translation of VM byte poke handler

#include "recovered.hpp"

// label: vm_op_ab_poke_byte

extern "C" void CB_NEAR cb_bloodprg_00684c_vm_op_ab_poke_byte(CbMachine* m)
{
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_advance_u16(m->si, 1, m->df);
    m->bx = m->read16(m->ds, m->si);
    m->write8(m->ds, m->bx, cb_lo8(m->ax));
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + 2);
    m->set_add16_flags(before_add, 2, m->si);
    return;
}
