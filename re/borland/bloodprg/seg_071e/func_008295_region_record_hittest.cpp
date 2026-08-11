// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x008295
// assembly: re/assembly/bloodprg/seg_071e/func_008295_region_record_hittest.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_region_record_hittest
// reason: mechanical translation of SS:BP rectangle hit test returning carry

#include "recovered.hpp"

// label: region_record_hittest

extern "C" void CB_FAR cb_bloodprg_008295_region_record_hittest(CbMachine* m)
{
    m->push16(m->ax);
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0a3e) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        m->cf = 0;
        m->ax = m->pop16();
        return;
    }
    m->ax = m->read16(m->ds, 0x0a2a);
    cb_u16 rect_x = m->read16(m->ss, m->bp);
    cb_u16 cmp_x_min = (cb_u16)(m->ax - rect_x);
    m->set_sub16_flags(m->ax, rect_x, cmp_x_min);
    if ((cb_i16)m->ax < (cb_i16)rect_x) {
        m->cf = 0;
        m->ax = m->pop16();
        return;
    }
    cb_u16 width = m->read16(m->ss, (cb_u16)(m->bp + 4));
    cb_u16 before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - width);
    m->set_sub16_flags(before_sub, width, m->ax);
    rect_x = m->read16(m->ss, m->bp);
    cb_u16 cmp_x_max = (cb_u16)(m->ax - rect_x);
    m->set_sub16_flags(m->ax, rect_x, cmp_x_max);
    if ((cb_i16)m->ax > (cb_i16)rect_x) {
        m->cf = 0;
        m->ax = m->pop16();
        return;
    }
    m->ax = m->read16(m->ds, 0x0a2c);
    cb_u16 rect_y = m->read16(m->ss, (cb_u16)(m->bp + 2));
    cb_u16 cmp_y_min = (cb_u16)(m->ax - rect_y);
    m->set_sub16_flags(m->ax, rect_y, cmp_y_min);
    if ((cb_i16)m->ax < (cb_i16)rect_y) {
        m->cf = 0;
        m->ax = m->pop16();
        return;
    }
    cb_u16 height = m->read16(m->ss, (cb_u16)(m->bp + 6));
    before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - height);
    m->set_sub16_flags(before_sub, height, m->ax);
    rect_y = m->read16(m->ss, (cb_u16)(m->bp + 2));
    cb_u16 cmp_y_max = (cb_u16)(m->ax - rect_y);
    m->set_sub16_flags(m->ax, rect_y, cmp_y_max);
    if ((cb_i16)m->ax > (cb_i16)rect_y) {
        m->cf = 0;
        m->ax = m->pop16();
        return;
    }
    m->cf = 1;
    m->ax = m->pop16();
    return;
}
