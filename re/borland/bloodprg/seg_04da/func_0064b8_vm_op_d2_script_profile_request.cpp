// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0064b8
// assembly: re/assembly/bloodprg/seg_04da/func_0064b8_vm_op_d2_script_profile_request.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_d2_script_profile_request
// reason: mechanical translation of lodsb/cbw/dec plus GS profile request store

#include "recovered.hpp"

// label: vm_op_d2_script_profile_request

extern "C" void CB_NEAR cb_bloodprg_0064b8_vm_op_d2_script_profile_request(CbMachine* m)
{
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_advance_u16(m->si, 1, m->df);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    cb_u16 before_dec = m->ax;
    m->ax = (cb_u16)(m->ax - 1);
    m->set_dec16_flags(before_dec, m->ax);
    m->write16(m->gs, 0x6780, m->ax);
    return;
}
