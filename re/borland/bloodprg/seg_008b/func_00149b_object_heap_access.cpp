// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00149b
// assembly: re/assembly/bloodprg/seg_008b/func_00149b_object_heap_access.asm
// provenance: recursive_graph
// status: translated_object_heap_access
// reason: mechanical translation of object-heap flag test and activity-byte increment loop

#include "recovered.hpp"

// label: object_heap_access

extern "C" void CB_NEAR cb_bloodprg_00149b_object_heap_access(CbMachine* m)
{
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->es = m->read16(m->ds, 0x6726);
    cb_u16 lds_seg = m->ds;
    m->si = m->read16(lds_seg, 0x672c);
    m->ds = m->read16(lds_seg, 0x672e);
    for (;;) {
        m->di = m->read16(m->ds, (cb_u16)(m->si + 0x10));
        cb_u16 word_test = (cb_u16)(m->read16(m->es, m->di) & 0x0118u);
        m->set_logic16_flags(word_test);
        if (word_test != 0) {
            cb_u8 byte_test = (cb_u8)(m->read8(m->es, (cb_u16)(m->di + 2)) & 2);
            m->set_logic8_flags(byte_test);
            if (byte_test != 0) {
                cb_u16 activity_off = (cb_u16)(m->di + 0x14);
                cb_u8 before_inc8 = m->read8(m->es, activity_off);
                cb_u8 after_inc8 = (cb_u8)(before_inc8 + 1);
                m->write8(m->es, activity_off, after_inc8);
                m->set_inc8_flags(before_inc8, after_inc8);
            }
        }
        cb_u16 before_add = m->si;
        m->si = (cb_u16)(m->si + 0x14);
        m->set_add16_flags(before_add, 0x14, m->si);
        cb_u16 state = m->read16(m->ds, (cb_u16)(m->si + 0x12));
        cb_u16 cmp_result = (cb_u16)(state - 1);
        m->set_sub16_flags(state, 1, cmp_result);
        if (cmp_result != 0) {
            break;
        }
    }
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    return;
}
