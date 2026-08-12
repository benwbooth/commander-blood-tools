/* Codegen probe for the MANU3 cumulative segment relocation protocol. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define FAR_AT(type, segment, offset) \
    ((type FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA FAR
#endif

typedef struct segment_directory {
    u16 field_000;
    u16 work_segment_0;
    u16 work_segment_1;
    u16 work_segment_2;
    u16 field_008;
    u16 field_00a;
    u16 work_delta_0;
    u16 work_delta_1;
    u16 work_delta_2;
} segment_directory;

extern volatile u16 CODE_DATA data_segment_delta;
extern volatile u16 CODE_DATA active_data_segment;

void FAR xdb_manu3_init_protocol_probe(u16 code_segment);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_init_protocol_probe \
        parm [ax] modify exact [ax bx cx dx si di bp]
#endif

void FAR xdb_manu3_init_protocol_probe(u16 code_segment)
{
    volatile segment_directory FAR *directory;
    u16 segment;

    segment = (u16)(code_segment + data_segment_delta);
    active_data_segment = segment;
    directory = FAR_AT(volatile segment_directory, segment, 0);

    segment = (u16)(segment + directory->work_delta_0);
    directory->work_segment_0 = segment;
    segment = (u16)(segment + directory->work_delta_1);
    directory->work_segment_1 = segment;
    segment = (u16)(segment + directory->work_delta_2);
    directory->work_segment_2 = segment;

    *FAR_AT(volatile u16, segment, 0x067e) = 0x0ae0u;
}
