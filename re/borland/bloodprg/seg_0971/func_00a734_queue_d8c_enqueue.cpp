// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a734
// assembly: re/assembly/bloodprg/seg_0971/func_00a734_queue_d8c_enqueue.asm
// provenance: recursive_graph
// status: translated_queue_d8c_enqueue
// reason: mechanical translation of two DS queue adds plus CLC

#include "recovered.hpp"

// label: queue_d8c_enqueue

extern "C" void CB_NEAR cb_bloodprg_00a734_queue_d8c_enqueue(CbMachine* m)
{
    cb_u16 head = m->read16(m->ds, 0x0d8c);
    cb_u16 head_result = (cb_u16)(head + m->ax);
    m->write16(m->ds, 0x0d8c, head_result);
    m->set_add16_flags(head, m->ax, head_result);
    cb_u16 count = m->read16(m->ds, 0x0d9a);
    cb_u16 count_result = (cb_u16)(count + m->ax);
    m->write16(m->ds, 0x0d9a, count_result);
    m->set_add16_flags(count, m->ax, count_result);
    m->cf = 0;
    return;
}
