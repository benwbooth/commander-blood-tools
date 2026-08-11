// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a1f3
// assembly: re/assembly/bloodprg/seg_0971/func_00a1f3_routine.asm
// provenance: recursive_graph
// status: translated_list_d8c_iterate_epilogue
// reason: mechanical translation of list iterate gate plus saved-register epilogue

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00a1f3_routine(CbMachine* m)
{
    cb_set_lo8(m->ax, m->read8(m->ds, 0x0d76));
    cb_u8 masked = (cb_u8)(cb_lo8(m->ax) & 0x80u);
    cb_set_lo8(m->ax, masked);
    m->set_logic8_flags(masked);
    m->write8(m->ds, 0x0dac, masked);
    m->call_near(0xa2ab);
    m->write8(m->ds, 0x0dac, 0);
    m->bp = m->pop16();
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    return;
}
