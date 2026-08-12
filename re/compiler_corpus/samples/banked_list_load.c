/*
 * Codegen probe for BLOODPRG 0x00A642.
 *
 * The original communicates helper results through carry, AX, and ES:SI.
 * This probe deliberately uses ordinary C returns and output parameters so
 * the compiler experiment measures the cost of the natural representation.
 */
typedef unsigned char u8;
typedef unsigned short u16;

#if defined(__WATCOMC__)
#define FAR __far
#define NEAR __near
#define FAR_FN __far
#elif defined(__TURBOC__)
#define FAR far
#define NEAR near
#define FAR_FN far
#else
#define FAR
#define NEAR
#define FAR_FN
#endif

extern volatile u16 list_d8c_head_offset;
extern volatile u16 list_d8c_tail_offset;
extern volatile u16 list_d8c_buffer_end_offset;
extern volatile u8 FAR list_d8c_buffer[];

extern void FAR_FN list_d8c_init_probe(void);
extern int NEAR list_d8c_read_probe(u16 *entry_extent, u16 *cursor_offset);
extern int NEAR ems_paged_read_probe(u16 byte_count);

int NEAR banked_list_load_probe(void)
{
    u16 entry_extent;
    u16 cursor_offset;
    u16 entry_start;

    list_d8c_init_probe();
    if (!list_d8c_read_probe(&entry_extent, &cursor_offset)) {
        return 0;
    }

    entry_start = (u16)(list_d8c_buffer_end_offset - entry_extent - 2u);
    list_d8c_tail_offset = entry_start;
    *(volatile u16 FAR *)(list_d8c_buffer + entry_start) = entry_extent;
    entry_start += 2u;
    list_d8c_head_offset = entry_start;

    return ems_paged_read_probe((u16)(entry_extent - 2u));
}
