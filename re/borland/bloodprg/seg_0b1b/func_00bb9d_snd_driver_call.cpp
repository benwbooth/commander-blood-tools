// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00bb9d
// assembly: re/assembly/bloodprg/seg_0b1b/func_00bb9d_snd_driver_call.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_snd_driver_call
// reason: mechanical translation of GS-based indirect sound-driver far call

#include "recovered.hpp"

// label: snd_driver_call

extern "C" void CB_FAR cb_bloodprg_00bb9d_snd_driver_call(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->ds);
    m->push16(m->es);
    m->ax = m->gs;
    m->ds = m->ax;
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    cb_u16 target_off = m->read16(m->ds, 0x0cdf);
    cb_u16 target_seg = m->read16(m->ds, 0x0ce1);
    m->call_far(target_seg, target_off);
    m->write8(m->ds, 0x0ba0, 0);
    m->es = m->pop16();
    m->ds = m->pop16();
    m->ax = m->pop16();
    return;
}
