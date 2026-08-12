#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_face_builder_next(void)
{
    xdb_manu3_face_bucket_sort(
            xdb_manu3_segments.work_segment_0,
            xdb_manu3_segments.work_segment_2);
}
