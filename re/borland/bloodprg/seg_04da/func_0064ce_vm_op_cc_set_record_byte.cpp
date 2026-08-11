// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0064ce
// assembly: re/assembly/bloodprg/seg_04da/func_0064ce_vm_op_cc_set_record_byte.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_cc_set_record_byte
// reason: mechanical translation of BP-indexed byte copy loop

#include "recovered.hpp"

// label: vm_op_cc_set_record_byte

extern "C" void CB_NEAR cb_bloodprg_0064ce_vm_op_cc_set_record_byte(CbMachine* m)
{
    m->bp = 0x6cde;
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_advance_u16(m->si, 1, m->df);
    cb_set_lo8(m->ax, (cb_u8)(cb_lo8(m->ax) - 1));
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    m->ax = (cb_u16)(m->ax << 4);
    m->bp = (cb_u16)(m->bp + m->ax);
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        m->write8(m->ss, m->bp, cb_lo8(m->ax));
        m->bp = (cb_u16)(m->bp + 1);
        cb_u8 test_result = cb_lo8(m->ax);
        m->set_logic8_flags(test_result);
        if (test_result == 0) {
            cb_u16 before_inc = m->si;
            m->si = (cb_u16)(m->si + 1);
            m->set_inc16_flags(before_inc, m->si);
            return;
        }
    }
}
