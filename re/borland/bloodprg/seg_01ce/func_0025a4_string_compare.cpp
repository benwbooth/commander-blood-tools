// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0025a4
// assembly: re/assembly/bloodprg/seg_01ce/func_0025a4_string_compare.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_string_compare
// reason: mechanical translation of saved-register byte string compare

#include "recovered.hpp"

// label: string_compare

extern "C" void CB_FAR cb_bloodprg_0025a4_string_compare(CbMachine* m)
{
    cb_u16 saved_ax = m->ax;
    cb_u16 saved_si = m->si;
    cb_u16 saved_di = m->di;

    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        cb_u8 left = cb_lo8(m->ax);
        cb_u8 right = m->read8(m->es, m->di);
        cb_u8 cmp_result = (cb_u8)(left - right);
        m->set_sub8_flags(left, right, cmp_result);
        if (cmp_result != 0) {
            m->cf = 0;
            m->di = saved_di;
            m->si = saved_si;
            m->ax = saved_ax;
            return;
        }
        m->di = (cb_u16)(m->di + 1);
        m->set_logic8_flags(left);
        if (left == 0) {
            m->cf = 1;
            m->di = saved_di;
            m->si = saved_si;
            m->ax = saved_ax;
            return;
        }
    }
}
