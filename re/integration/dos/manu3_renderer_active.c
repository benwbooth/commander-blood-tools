#include <dos.h>
#include <stdio.h>

#include "xdb_manu3.h"

#define GEOMETRY_BYTES 0x2000u
#define RASTER_BYTES 0x6000u
#define FRAMEBUFFER_BYTES 64000ul
#define FACE_OFFSET 0x1000u
#define VERTEX_0_OFFSET 0x1100u
#define VERTEX_1_OFFSET 0x1200u
#define VERTEX_2_OFFSET 0x1300u
#define RESULT_FILE "RESULT.TXT"
#define FRAME_FILE "FRAME.BIN"

volatile xdb_manu3_segment_directory xdb_manu3_segments;
volatile xdb_u16 xdb_manu3_linear_framebuffer_segment;
volatile xdb_u16 xdb_manu3_framebuffer_segment;
volatile xdb_u16 xdb_manu3_face_list_offset;
volatile xdb_u16 xdb_manu3_face_count;

static void fill_segment(xdb_u16 segment, xdb_u16 size, xdb_u8 value)
{
    volatile xdb_u8 XDB_FAR *bytes = XDB_FAR_AT(
            volatile xdb_u8,
            segment,
            0u);
    xdb_u16 offset;

    for (offset = 0u; offset < size; ++offset) {
        bytes[offset] = value;
    }
}

static void fill_full_segment(xdb_u16 segment, xdb_u8 multiplier, xdb_u8 addend)
{
    volatile xdb_u8 XDB_FAR *bytes = XDB_FAR_AT(
            volatile xdb_u8,
            segment,
            0u);
    xdb_u16 offset = 0u;

    do {
        bytes[offset] = (xdb_u8)(offset * multiplier + addend);
        ++offset;
    } while (offset != 0u);
}

static xdb_u32 checksum_segment(xdb_u16 segment, xdb_u16 size)
{
    const volatile xdb_u8 XDB_FAR *bytes = XDB_FAR_AT(
            const volatile xdb_u8,
            segment,
            0u);
    xdb_u16 offset;
    xdb_u32 checksum = 0u;

    for (offset = 0u; offset < size; ++offset) {
        checksum = checksum * 33u + bytes[offset];
    }
    return checksum;
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

static int write_framebuffer(xdb_u16 framebuffer_segment)
{
    const volatile xdb_u8 XDB_FAR *framebuffer = XDB_FAR_AT(
            const volatile xdb_u8,
            framebuffer_segment,
            0u);
    FILE *output = fopen(FRAME_FILE, "wb");
    xdb_u32 offset;

    if (output == NULL) {
        return 0;
    }
    for (offset = 0u; offset < FRAMEBUFFER_BYTES; ++offset) {
        if (fputc(framebuffer[(xdb_u16)offset], output) == EOF) {
            fclose(output);
            return 0;
        }
    }
    if (fclose(output) != 0) {
        return 0;
    }
    return 1;
}

int main(void)
{
    xdb_u16 geometry_segment;
    xdb_u16 raster_segment;
    xdb_u16 texture_segment;
    xdb_u16 framebuffer_segment;
    xdb_u16 nonzero_pixels = 0u;
    xdb_u32 geometry_checksum;
    xdb_u32 offset;
    volatile xdb_manu3_face XDB_FAR *face;
    volatile xdb_manu3_vertex XDB_FAR *vertex_0;
    volatile xdb_manu3_vertex XDB_FAR *vertex_1;
    volatile xdb_manu3_vertex XDB_FAR *vertex_2;
    volatile xdb_i32 XDB_FAR *reciprocal_table;
    volatile xdb_u16 XDB_FAR *continuation;
    const volatile xdb_u8 XDB_FAR *framebuffer;

    if (_dos_allocmem(GEOMETRY_BYTES / 16u, &geometry_segment) != 0u) {
        return write_result("FAIL geometry allocation");
    }
    if (_dos_allocmem(RASTER_BYTES / 16u, &raster_segment) != 0u) {
        return write_result("FAIL raster allocation");
    }
    if (_dos_allocmem(0x1000u, &texture_segment) != 0u) {
        return write_result("FAIL texture allocation");
    }
    if (_dos_allocmem(0x1000u, &framebuffer_segment) != 0u) {
        return write_result("FAIL framebuffer allocation");
    }

    fill_segment(geometry_segment, GEOMETRY_BYTES, 0u);
    fill_segment(raster_segment, RASTER_BYTES, 0u);
    fill_full_segment(texture_segment, 37u, 11u);
    fill_full_segment(framebuffer_segment, 0u, 0u);

    reciprocal_table = XDB_FAR_AT(
            volatile xdb_i32,
            raster_segment,
            0u);
    reciprocal_table[2] = 0x00008000l;
    reciprocal_table[4] = 0x00004000l;
    continuation = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_RENDER_CONTINUATION_OFFSET);
    *continuation = XDB_MANU3_RENDER_LINEAR_OFFSET;

    face = XDB_FAR_AT(
            volatile xdb_manu3_face,
            geometry_segment,
            FACE_OFFSET);
    vertex_0 = XDB_FAR_AT(
            volatile xdb_manu3_vertex,
            geometry_segment,
            VERTEX_0_OFFSET);
    vertex_1 = XDB_FAR_AT(
            volatile xdb_manu3_vertex,
            geometry_segment,
            VERTEX_1_OFFSET);
    vertex_2 = XDB_FAR_AT(
            volatile xdb_manu3_vertex,
            geometry_segment,
            VERTEX_2_OFFSET);
    face->vertex_0 = VERTEX_0_OFFSET;
    face->vertex_1 = VERTEX_1_OFFSET;
    face->vertex_2 = VERTEX_2_OFFSET;

    vertex_0->link = 0x0123u;
    vertex_0->field_002 = 0x1234u;
    vertex_0->screen.position.x = 10;
    vertex_0->screen.position.y = 20;
    vertex_0->depth = 0x10203040l;
    vertex_0->clip_flags = 0u;

    vertex_1->link = 0x2345u;
    vertex_1->field_002 = 0x3456u;
    vertex_1->screen.position.x = 10;
    vertex_1->screen.position.y = 24;
    vertex_1->depth = -0x01234567l;
    vertex_1->clip_flags = 0u;

    vertex_2->link = 0x4567u;
    vertex_2->field_002 = 0x5678u;
    vertex_2->screen.position.x = 12;
    vertex_2->screen.position.y = 20;
    vertex_2->depth = 0x30405060l;
    vertex_2->clip_flags = 0u;

    xdb_manu3_segments.work_segment_1 = (xdb_u16)(
            texture_segment - 0x2000u);
    xdb_manu3_linear_framebuffer_segment = framebuffer_segment;
    xdb_manu3_framebuffer_segment = framebuffer_segment;
    xdb_manu3_face_list_offset = FACE_OFFSET;
    xdb_manu3_face_count = 1u;
    geometry_checksum = checksum_segment(geometry_segment, GEOMETRY_BYTES);

    xdb_manu3_face_bucket_sort(geometry_segment, raster_segment);

    if (checksum_segment(geometry_segment, GEOMETRY_BYTES)
            != geometry_checksum) {
        return write_result("FAIL geometry changed");
    }
    if (*XDB_FAR_AT(volatile xdb_u16, raster_segment, 0x0908u)
            != XDB_MANU3_RASTER_POOL_OFFSET) {
        return write_result("FAIL raster record not returned");
    }

    framebuffer = XDB_FAR_AT(
            const volatile xdb_u8,
            framebuffer_segment,
            0u);
    for (offset = 0u; offset < FRAMEBUFFER_BYTES; ++offset) {
        if (framebuffer[(xdb_u16)offset] != 0u) {
            ++nonzero_pixels;
        }
    }
    if (nonzero_pixels != 4u) {
        return write_result("FAIL framebuffer pixel count");
    }
    if (!write_framebuffer(framebuffer_segment)) {
        return write_result("FAIL framebuffer write");
    }

    _dos_freemem(framebuffer_segment);
    _dos_freemem(texture_segment);
    _dos_freemem(raster_segment);
    _dos_freemem(geometry_segment);
    return write_result("PASS manu3 renderer active");
}
