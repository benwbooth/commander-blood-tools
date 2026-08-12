/*
 * Codegen probe for BLOODPRG 0x00A3AD.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_tail_offset;
extern volatile u16 list_d8c_byte_count;
extern volatile u16 list_d8c_wrap_limit;

int NEAR queue_d8c_has_room_probe(u16 byte_count)
{
    u16 head;
    u16 tail;
    u16 needed;

    head = list_d8c_head_offset;
    tail = list_d8c_tail_offset;
    if (head < tail) {
        needed = (u16)(head + byte_count);
        needed = (u16)(needed + 0x12u);
        if (tail < needed) {
            return 0;
        }
    }

    needed = (u16)(list_d8c_byte_count + 0x0au);
    needed = (u16)(needed + byte_count);
    if (needed < byte_count) {
        return 0;
    }

    return list_d8c_wrap_limit >= needed;
}
