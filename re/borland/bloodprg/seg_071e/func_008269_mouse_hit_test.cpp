// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x008269
// assembly: re/assembly/bloodprg/seg_071e/func_008269_mouse_hit_test.asm
// provenance: recursive_graph
// status: translated_mouse_hit_test
// reason: mechanical translation of DS:SI rectangle hit test and SS:BP flag OR

#include "recovered.hpp"

// label: mouse_hit_test

extern "C" void CB_NEAR cb_bloodprg_008269_mouse_hit_test(CbMachine* m)
{
    m->push16(m->ax);
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0a3e) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        m->ax = m->pop16();
        return;
    }
    m->ax = m->read16(m->ds, 0x0a2a);
    cb_u16 rect_x = m->read16(m->ds, m->si);
    cb_u16 cmp_x_min = (cb_u16)(m->ax - rect_x);
    m->set_sub16_flags(m->ax, rect_x, cmp_x_min);
    if ((cb_i16)m->ax < (cb_i16)rect_x) {
        m->ax = m->pop16();
        return;
    }
    cb_u16 width = m->read16(m->ds, (cb_u16)(m->si + 4));
    cb_u16 before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - width);
    m->set_sub16_flags(before_sub, width, m->ax);
    rect_x = m->read16(m->ds, m->si);
    cb_u16 cmp_x_max = (cb_u16)(m->ax - rect_x);
    m->set_sub16_flags(m->ax, rect_x, cmp_x_max);
    if ((cb_i16)m->ax > (cb_i16)rect_x) {
        m->ax = m->pop16();
        return;
    }
    m->ax = m->read16(m->ds, 0x0a2c);
    cb_u16 rect_y = m->read16(m->ds, (cb_u16)(m->si + 2));
    cb_u16 cmp_y_min = (cb_u16)(m->ax - rect_y);
    m->set_sub16_flags(m->ax, rect_y, cmp_y_min);
    if ((cb_i16)m->ax < (cb_i16)rect_y) {
        m->ax = m->pop16();
        return;
    }
    cb_u16 height = m->read16(m->ds, (cb_u16)(m->si + 6));
    before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - height);
    m->set_sub16_flags(before_sub, height, m->ax);
    rect_y = m->read16(m->ds, (cb_u16)(m->si + 2));
    cb_u16 cmp_y_max = (cb_u16)(m->ax - rect_y);
    m->set_sub16_flags(m->ax, rect_y, cmp_y_max);
    if ((cb_i16)m->ax > (cb_i16)rect_y) {
        m->ax = m->pop16();
        return;
    }
    cb_u8 flags = (cb_u8)(m->read8(m->ss, m->bp) | 8);
    m->write8(m->ss, m->bp, flags);
    m->set_logic8_flags(flags);
    m->ax = m->pop16();
    return;
}
