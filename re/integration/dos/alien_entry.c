#include <dos.h>
#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta;
volatile xdb_u16 XDB_CODE_DATA xdb_alien_method_delta_high;
volatile xdb_u16 XDB_CODE_DATA xdb_croolis_data_segment_delta;
volatile xdb_u16 XDB_CODE_DATA xdb_croolis_data_segment;

static volatile xdb_u16 timing_scale = 7u;
static xdb_alien_api_request request;
static xdb_u16 allocated_segment;
static xdb_u16 main_calls;
static xdb_u16 main_data_segment;
static xdb_i16 main_method_delta;
static xdb_u16 main_method_delta_high;
static xdb_alien_frame_callback main_frame_callback;

static void XDB_FAR test_frame_callback(xdb_u16 event, xdb_u32 clock)
{
    (void)event;
    (void)clock;
}

void XDB_FAR xdb_croolis_main(void)
{
    volatile xdb_alien_segment_directory XDB_FAR *directory = XDB_FAR_AT(
            volatile xdb_alien_segment_directory,
            allocated_segment,
            0u);

    ++main_calls;
    main_data_segment = xdb_croolis_data_segment;
    main_method_delta = xdb_alien_method_delta;
    main_method_delta_high = xdb_alien_method_delta_high;
    main_frame_callback = directory->frame_callback;
    xdb_alien_method_delta = 0x1234;
}

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) {
        return 2;
    }
    fprintf(result, "%s\n", status);
    printf("%s\n", status);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

int main(void)
{
    volatile xdb_alien_segment_directory XDB_FAR *directory;
    xdb_u16 code_segment;

    if (_dos_allocmem(0x0100u, &allocated_segment) != 0u) {
        return write_result("FAIL segment allocation");
    }
    directory = XDB_FAR_AT(
            volatile xdb_alien_segment_directory,
            allocated_segment,
            0u);
    _fmemset((void XDB_FAR *)directory, 0, 0x1000u);

    code_segment = FP_SEG(xdb_croolis_api_entry);
    xdb_croolis_data_segment_delta = (xdb_u16)(
            allocated_segment - code_segment);
    request.timing_scale = XDB_FAR_AT(
            volatile xdb_u16,
            FP_SEG(&timing_scale),
            FP_OFF(&timing_scale));
    request.frame_callback = test_frame_callback;

    xdb_croolis_api_entry(
            XDB_FAR_AT(
                    const volatile xdb_alien_api_request,
                    FP_SEG(&request),
                    FP_OFF(&request)),
            code_segment);

    if (main_calls != 1u
            || main_data_segment != allocated_segment
            || (xdb_u16)main_method_delta != 0x0034u
            || main_method_delta_high != 0u) {
        _dos_freemem(allocated_segment);
        return write_result("FAIL main entry state");
    }
    if (directory->object_segment != allocated_segment
            || directory->palette_segment != allocated_segment
            || directory->raster_segment != allocated_segment) {
        _dos_freemem(allocated_segment);
        return write_result("FAIL segment directory");
    }
    if (*XDB_FAR_AT(
                volatile xdb_u16,
                allocated_segment,
                XDB_CROOLIS_RENDER_CONTINUATION_OFFSET)
            != XDB_CROOLIS_RENDER_MODE_X_OFFSET) {
        _dos_freemem(allocated_segment);
        return write_result("FAIL render continuation");
    }
    if (main_frame_callback != test_frame_callback
            || directory->frame_callback != test_frame_callback) {
        _dos_freemem(allocated_segment);
        return write_result("FAIL frame callback");
    }
    if (timing_scale != 0x0247u
            || (xdb_u16)xdb_alien_method_delta != 0x1234u
            || xdb_alien_method_delta_high != 0u) {
        _dos_freemem(allocated_segment);
        return write_result("FAIL timing writeback");
    }

    _dos_freemem(allocated_segment);
    return write_result("PASS alien entry");
}
