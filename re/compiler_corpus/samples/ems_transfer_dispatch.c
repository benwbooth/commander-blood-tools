/*
 * Codegen probe for BLOODPRG 0x00BD09.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 ems_transfer_mode;

void NEAR ems_map_page_and_copy(u16 value, volatile u8 FAR *destination);
void NEAR ems_buffer_setup(u16 value, volatile u8 FAR *destination);
void NEAR ems_page_offset_split(u16 value, volatile u8 FAR *destination);

#if defined(__WATCOMC__)
#pragma aux ems_map_page_and_copy parm [ax] [es di] modify exact []
#pragma aux ems_buffer_setup parm [ax] [es di] modify exact []
#pragma aux ems_page_offset_split parm [ax] [es di] modify exact []
#pragma aux ems_transfer_dispatch_probe parm [ax] [es di]
#endif

void NEAR ems_transfer_dispatch_probe(u16 value,
        volatile u8 FAR *destination)
{
    i8 mode;

    mode = (i8)ems_transfer_mode;
    if (--mode < 0) {
        ems_map_page_and_copy(value, destination);
    } else {
        if (--mode < 0) {
            ems_buffer_setup(value, destination);
        } else {
            ems_page_offset_split(value, destination);
        }
    }
}
