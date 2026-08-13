/* Codegen probe for the alien overlay's far host API entry. */
#include <dos.h>

typedef unsigned int xdb_u16;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;

#define XDB_FAR far
#define XDB_FAR_AT(type, segment, offset) \
    ((type XDB_FAR *)MK_FP((segment), (offset)))
#if defined(__WATCOMC__)
#define XDB_CODE_DATA __based(__segname("_CODE"))
#else
#define XDB_CODE_DATA XDB_FAR
#endif

typedef void XDB_FAR xdb_alien_frame_function(
        xdb_u16 event,
        xdb_u32 clock);
typedef xdb_alien_frame_function XDB_FAR *xdb_alien_frame_callback;

typedef struct xdb_alien_api_request {
    volatile xdb_u16 XDB_FAR *timing_scale;
    xdb_alien_frame_callback frame_callback;
} xdb_alien_api_request;

typedef struct xdb_alien_segment_directory {
    xdb_u16 field_000;
    xdb_u16 object_segment;
    xdb_u16 palette_segment;
    xdb_u16 raster_segment;
    xdb_u16 field_008;
    xdb_u16 field_00a;
    xdb_u16 object_segment_delta;
    xdb_u16 palette_segment_delta;
    xdb_u16 raster_segment_delta;
    xdb_u16 field_012;
    xdb_u16 field_014;
    xdb_u32 frame_clock;
    xdb_u32 last_callback_clock;
    xdb_u16 callback_countdown;
    xdb_alien_frame_callback frame_callback;
} xdb_alien_segment_directory;

extern volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_method_delta_high;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_data_segment_delta;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_data_segment;
extern void XDB_FAR xdb_alien_main(void);

void XDB_FAR xdb_alien_api_entry_probe(
        const volatile xdb_alien_api_request XDB_FAR *request,
        xdb_u16 code_segment)
{
    volatile xdb_alien_segment_directory XDB_FAR *directory;
    xdb_u16 segment;
    xdb_u16 scaled;

    segment = (xdb_u16)(code_segment + xdb_alien_data_segment_delta);
    xdb_alien_data_segment = segment;
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
    *XDB_FAR_AT(volatile xdb_u16, segment, 0x0946u) = 0x2940u;

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

    xdb_alien_main();

    scaled = (xdb_u16)((xdb_u16)xdb_alien_method_delta + 4u);
    *request->timing_scale = (xdb_u16)(scaled >> 3);
}
