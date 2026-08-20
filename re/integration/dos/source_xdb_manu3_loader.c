#include <dos.h>
#include <fcntl.h>
#include <io.h>
#include <stdio.h>

#include "xdb_manu3.h"

#ifndef XDB_IMAGE_BYTES
#error XDB_IMAGE_BYTES must be defined by the source-XDB integration driver
#endif
#ifndef XDB_DATA_PARAGRAPH
#error XDB_DATA_PARAGRAPH must be defined by the source-XDB integration driver
#endif
#ifndef XDB_DATA_STATE_OFFSET
#error XDB_DATA_STATE_OFFSET must be defined by the source-XDB integration driver
#endif

#define XDB_FILENAME "MANU3.XDB"
#define RESULT_FILENAME "RESULT.TXT"

extern void call_overlay(
        xdb_u16 overlay_segment,
        const void XDB_NEAR *request);

#pragma aux call_overlay = \
        "push bp" \
        "push ax" \
        "xor bx,bx" \
        "push bx" \
        "mov bx,sp" \
        "mov bp,si" \
        "call dword ptr ss:[bx]" \
        "add sp,4" \
        "pop bp" \
        parm [ax] [si] \
        modify exact [ax bx cx dx si di es]

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILENAME, "w");

    if (result == NULL) {
        return 2;
    }
    fprintf(result, "%s\n", status);
    printf("%s\n", status);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static int load_overlay(xdb_u16 segment)
{
    int handle;
    unsigned error;
    unsigned long position = 0ul;

    error = _dos_open(XDB_FILENAME, O_RDONLY | O_BINARY, &handle);
    if (error != 0u) {
        return 0;
    }
    while (position < (unsigned long)XDB_IMAGE_BYTES) {
        union REGS registers;
        struct SREGS segments;
        unsigned long remaining = (unsigned long)XDB_IMAGE_BYTES - position;
        xdb_u16 count = remaining > 0xfff0ul
                ? 0xfff0u
                : (xdb_u16)remaining;

        registers.x.ax = 0x3f00u;
        registers.x.bx = (xdb_u16)handle;
        registers.x.cx = count;
        registers.x.dx = (xdb_u16)(position & 0x0ful);
        segread(&segments);
        segments.ds = (xdb_u16)(segment + (xdb_u16)(position >> 4));
        int86x(0x21, &registers, &registers, &segments);
        if (registers.x.cflag != 0u || registers.x.ax != count) {
            _dos_close(handle);
            return 0;
        }
        position += count;
    }
    _dos_close(handle);
    return 1;
}

int main(void)
{
    xdb_manu3_api_request request;
    volatile xdb_manu3_segment_directory XDB_FAR *directory;
    xdb_u16 overlay_segment;
    xdb_u16 data_segment;
    xdb_u16 expected_work_0;
    xdb_u16 expected_work_1;
    xdb_u16 expected_work_2;
    xdb_u16 paragraphs = (xdb_u16)(
            ((unsigned long)XDB_IMAGE_BYTES + 15ul) >> 4);

    if (_dos_allocmem(paragraphs, &overlay_segment) != 0u) {
        return write_result("FAIL source XDB allocation");
    }
    if (!load_overlay(overlay_segment)) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source XDB load");
    }

    request.cursor.x = 160;
    request.cursor.y = 100;
    request.animation_selector = 0u;
    request.framebuffer_window_offset = 0u;
    call_overlay(overlay_segment, &request);

    data_segment = (xdb_u16)(overlay_segment + XDB_DATA_PARAGRAPH);
    directory = XDB_FAR_AT(
            volatile xdb_manu3_segment_directory,
            data_segment,
            0u);
    expected_work_0 = (xdb_u16)(data_segment + directory->work_delta_0);
    expected_work_1 = (xdb_u16)(expected_work_0 + directory->work_delta_1);
    expected_work_2 = (xdb_u16)(expected_work_1 + directory->work_delta_2);

    if (*XDB_FAR_AT(
                volatile xdb_u16,
                overlay_segment,
                XDB_DATA_STATE_OFFSET) != data_segment) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source XDB data publication");
    }
    if (directory->work_segment_0 != expected_work_0
            || directory->work_segment_1 != expected_work_1
            || directory->work_segment_2 != expected_work_2) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source XDB segment directory");
    }
    if (*XDB_FAR_AT(
                volatile xdb_u16,
                expected_work_2,
                XDB_MANU3_RENDER_CONTINUATION_OFFSET)
            != 0x0ae0u) {
        _dos_freemem(overlay_segment);
        return write_result("FAIL source XDB render continuation");
    }

    _dos_freemem(overlay_segment);
    return write_result("PASS source-linked MANU3 XDB");
}
