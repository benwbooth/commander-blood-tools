// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x009b67
// assembly: re/assembly/bloodprg/seg_071e/func_009b67_ship_3d_point_cloud_randomize.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_ship_3d_point_cloud_randomize
// reason: mechanical translation of 1000-record GS point-cloud random initializer

#include "recovered.hpp"

// label: ship_3d_point_cloud_randomize

extern "C" void CB_FAR cb_bloodprg_009b67_ship_3d_point_cloud_randomize(CbMachine* m)
{
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ax);
    m->cx = 0x03e8;
    m->ax = m->gs;
    m->es = m->ax;
    m->di = 0x2fc1;
    while (m->cx != 0) {
        m->ax = 0xffff;
        m->call_far(0x01ce, 0x0b02);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
        m->ax = 0xffff;
        m->call_far(0x01ce, 0x0b02);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
        m->ax = 0xffff;
        m->call_far(0x01ce, 0x0b02);
        m->write16(m->es, m->di, m->ax);
        cb_advance_u16(m->di, 2, m->df);
        cb_u16 before_add = m->di;
        m->di = (cb_u16)(m->di + 2);
        m->set_add16_flags(before_add, 2, m->di);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->ax = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    return;
}
