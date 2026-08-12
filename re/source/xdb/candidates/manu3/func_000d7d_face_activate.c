#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_face_activate(
        const volatile xdb_manu3_face XDB_FAR *face)
{
    xdb_u16 vertex_0 = face->vertex_0;
    xdb_u16 vertex_1 = face->vertex_1;
    xdb_u16 vertex_2 = face->vertex_2;
    volatile void XDB_NEAR *raster =
            (volatile void XDB_NEAR *)xdb_manu3_active_raster_offset;

    if (raster != 0) {
        xdb_manu3_gradient_setup(
                vertex_0, vertex_1, vertex_2, raster);
    }
}
