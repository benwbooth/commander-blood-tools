// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000d61
// assembly: re/assembly/bloodprg/seg_0000/func_000d61_print_string_dos.asm
// provenance: recursive_graph
// status: translated_print_string_dos
// reason: mechanical translation of DOS int 21h character-output loop

#include "recovered.hpp"

// label: print_string_dos

extern "C" void CB_FAR cb_bloodprg_000d61_print_string_dos(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->dx);
    m->push16(m->si);
    cb_set_hi8(m->ax, 2);
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        m->set_logic8_flags(cb_lo8(m->ax));
        if (cb_lo8(m->ax) == 0) {
            break;
        }
        cb_set_lo8(m->dx, cb_lo8(m->ax));
        m->interrupt(0x21);
    }
    m->si = m->pop16();
    m->dx = m->pop16();
    m->ax = m->pop16();
    return;
}
