#include "../include/xdb_alien.h"

void XDB_FAR xdb_scrut_api_entry(
        const volatile xdb_alien_api_request XDB_FAR *request,
        xdb_u16 code_segment)
{
    volatile xdb_alien_segment_directory XDB_FAR *directory;
    xdb_u16 segment;
    xdb_u16 scaled;

    segment = (xdb_u16)(code_segment + xdb_scrut_data_segment_delta);
    xdb_scrut_data_segment = segment;
    directory = XDB_FAR_AT(
            volatile xdb_alien_segment_directory,
            segment,
            0u);

    segment = (xdb_u16)(segment + directory->object_segment_delta);
    directory->object_segment = segment;
    segment = (xdb_u16)(segment + directory->palette_segment_delta);
    directory->palette_segment = segment;
    segment = (xdb_u16)(segment + directory->raster_segment_delta);
    directory->raster_segment = segment;
    *XDB_FAR_AT(
            volatile xdb_u16,
            segment,
            XDB_SCRUT_RENDER_CONTINUATION_OFFSET) =
            XDB_SCRUT_RENDER_MODE_X_OFFSET;

    scaled = (xdb_u16)(*request->timing_scale << 3);
    if ((xdb_i16)scaled < 0) {
        scaled = 0u;
    }
    if (scaled >= 0x0080u) {
        scaled = 0x007fu;
    }
    xdb_alien_method_delta = (xdb_i16)(scaled - 4u);
    xdb_alien_method_delta_high = 0u;
    directory->frame_callback = request->frame_callback;

    xdb_scrut_main();

    scaled = (xdb_u16)((xdb_u16)xdb_alien_method_delta + 4u);
    *request->timing_scale = (xdb_u16)(scaled >> 3);
}
