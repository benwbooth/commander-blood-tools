// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00b7b0
// assembly: re/assembly/bloodprg/seg_0b1b/func_00b7b0_audio_param_init_cd5.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_audio_param_init_cd5
// reason: mechanical translation of audio parameter fill plus indirect driver callback

#include "recovered.hpp"

// label: audio_param_init_cd5

extern "C" void CB_FAR cb_bloodprg_00b7b0_audio_param_init_cd5(CbMachine* m)
{
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->dx);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->push16(m->bp);
    m->bx = m->gs;
    m->ds = m->bx;
    m->di = 0x0cd5;
    m->cx = 9;
    while (m->cx != 0) {
        m->write16(m->ds, m->di, m->ax);
        cb_u16 before_add = m->di;
        m->di = (cb_u16)(m->di + 4);
        m->set_add16_flags(before_add, 4, m->di);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->ax = 0x011d;
    m->write16(m->ds, 0x0aec, m->ax);
    m->write16(m->ds, 0x0aee, m->cs);
    m->ax = m->read16(m->ds, 0x0c45);
    cb_u16 target_off = m->read16(m->ds, 0x0cd3);
    cb_u16 target_seg = m->read16(m->ds, 0x0cd5);
    m->call_far(target_seg, target_off);
    m->bp = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    return;
}
