/*
 * Codegen probe for BLOODPRG 0x00A622.
 *
 * This deliberately uses ordinary output parameters for the original
 * carry/AX/ES:SI result boundary. The body remains natural C and lets the
 * compiler experiment show the cost of that representation.
 */
typedef unsigned char u8;
typedef unsigned short u16;

#if defined(__WATCOMC__)
#define FAR __far
#define NEAR __near
#elif defined(__TURBOC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u16 list_d8c_head_offset;
extern volatile u8 FAR list_d8c_buffer[];
extern int NEAR ems_paged_read_probe(u16 byte_count);

int NEAR list_d8c_read_probe(u16 *entry_extent, u16 *cursor_offset)
{
    u16 cursor;

    if (!ems_paged_read_probe(2u)) {
        return 0;
    }

    cursor = list_d8c_head_offset;
    *cursor_offset = cursor;
    *entry_extent = *(volatile u16 FAR *)
            (list_d8c_buffer + (u16)(cursor - 2u));
    return 1;
}
