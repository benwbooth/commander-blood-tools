// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a141
// assembly: re/assembly/bloodprg/seg_0971/func_00a141_close_file_d5b.asm
// provenance: recursive_graph
// status: translated_close_file_d5b
// reason: mechanical translation of DOS close gate plus list bound reset call

#include "recovered.hpp"

// label: close_file_d5b

extern "C" void CB_NEAR cb_bloodprg_00a141_close_file_d5b(CbMachine* m)
{
    m->bx = m->read16(m->ds, 0x0d5b);
    m->set_logic16_flags(m->bx);
    if (m->bx != 0) {
        cb_u16 reserved = m->read16(m->ds, 0x0a86);
        cb_u16 cmp_result = (cb_u16)(m->bx - reserved);
        m->set_sub16_flags(m->bx, reserved, cmp_result);
        if (cmp_result != 0) {
            m->write16(m->ds, 0x0d5b, 0);
            cb_set_hi8(m->ax, 0x3e);
            m->interrupt(0x21);
            m->call_near(0xa73e);
        }
    }
    m->cx = 0;
    m->set_logic16_flags(m->cx);
    return;
}
