#include "../include/xdb_manu3.h"

void XDB_FAR xdb_manu3_init_protocol(xdb_u16 code_segment)
{
    volatile xdb_manu3_segment_directory XDB_FAR *directory;
    xdb_u16 segment;

    segment = (xdb_u16)(code_segment + xdb_manu3_data_segment_delta);
    xdb_manu3_data_segment = segment;
    directory = XDB_FAR_AT(
            volatile xdb_manu3_segment_directory, segment, 0);

    segment = (xdb_u16)(segment + directory->work_delta_0);
    directory->work_segment_0 = segment;
    segment = (xdb_u16)(segment + directory->work_delta_1);
    directory->work_segment_1 = segment;
    segment = (xdb_u16)(segment + directory->work_delta_2);
    directory->work_segment_2 = segment;

    *XDB_FAR_AT(volatile xdb_u16, segment, 0x067e) = 0x0ae0u;
}
