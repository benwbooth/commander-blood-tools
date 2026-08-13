#include <dos.h>
#include <stdio.h>

#include "xdb_manu3.h"

#define GEOMETRY_BYTES 0x1000u
#define RASTER_BYTES 0x6000u
#define FACE_OFFSET 0x0100u
#define VERTEX_0_OFFSET 0x0200u
#define VERTEX_1_OFFSET 0x0220u
#define VERTEX_2_OFFSET 0x0240u
#define INITIAL_BYTE 0xccu
#define RESULT_FILE "RESULT.TXT"

volatile xdb_manu3_segment_directory xdb_manu3_segments;
volatile xdb_u16 xdb_manu3_linear_framebuffer_segment;
volatile xdb_u16 xdb_manu3_framebuffer_segment;
volatile xdb_u16 xdb_manu3_active_raster_offset;
volatile xdb_u16 xdb_manu3_face_list_offset;
volatile xdb_u16 xdb_manu3_face_count;
const volatile xdb_i32 xdb_manu3_reciprocal_table[401];

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

static xdb_u8 expected_raster_byte(xdb_u16 offset)
{
    xdb_u16 pool_end = (xdb_u16)(
            XDB_MANU3_RASTER_POOL_OFFSET
            + XDB_MANU3_RASTER_POOL_COUNT
                    * sizeof(xdb_manu3_raster_record));

    if (offset == XDB_MANU3_COLUMN_OFFSET
            || offset == XDB_MANU3_COLUMN_OFFSET + 1u) {
        return 0u;
    }
    if (offset == XDB_MANU3_BUCKET_CURSOR_OFFSET) {
        return (xdb_u8)XDB_MANU3_BUCKET_HEADS_OFFSET;
    }
    if (offset == XDB_MANU3_BUCKET_CURSOR_OFFSET + 1u) {
        return (xdb_u8)(XDB_MANU3_BUCKET_HEADS_OFFSET >> 8);
    }
    if (offset >= XDB_MANU3_BUCKET_HEADS_OFFSET
            && offset < XDB_MANU3_BUCKET_HEADS_OFFSET
                    + XDB_MANU3_SCREEN_WIDTH * sizeof(xdb_u16)) {
        return 0u;
    }
    if (offset == 0x0908u) {
        return (xdb_u8)XDB_MANU3_RASTER_POOL_OFFSET;
    }
    if (offset == 0x0909u) {
        return (xdb_u8)(XDB_MANU3_RASTER_POOL_OFFSET >> 8);
    }
    if (offset >= XDB_MANU3_RASTER_POOL_OFFSET && offset < pool_end) {
        xdb_u16 relative = (xdb_u16)(
                offset - XDB_MANU3_RASTER_POOL_OFFSET);
        xdb_u16 field_offset = (xdb_u16)(
                relative % sizeof(xdb_manu3_raster_record));

        if (field_offset < sizeof(xdb_u16)) {
            xdb_u16 record_offset = (xdb_u16)(offset - field_offset);
            xdb_u16 next_offset = (xdb_u16)(
                    record_offset + sizeof(xdb_manu3_raster_record));

            if (next_offset == pool_end) {
                next_offset = 0u;
            }
            if (field_offset == 0u) {
                return (xdb_u8)next_offset;
            }
            return (xdb_u8)(next_offset >> 8);
        }
    }
    return INITIAL_BYTE;
}

static int write_result(const char *status, xdb_u16 offset)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) {
        return 2;
    }
    if (offset == 0xffffu) {
        fprintf(result, "%s\n", status);
        printf("%s\n", status);
    } else {
        fprintf(result, "%s at %04x\n", status, offset);
        printf("%s at %04x\n", status, offset);
    }
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static int write_mismatch(xdb_u16 offset, xdb_u8 actual, xdb_u8 expected)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) {
        return 2;
    }
    fprintf(
            result,
            "FAIL raster mismatch at %04x: got %02x expected %02x\n",
            offset,
            actual,
            expected);
    printf(
            "FAIL raster mismatch at %04x: got %02x expected %02x\n",
            offset,
            actual,
            expected);
    fclose(result);
    return 1;
}

int main(void)
{
    xdb_u16 geometry_segment;
    xdb_u16 raster_segment;
    xdb_u16 offset;
    xdb_u32 geometry_checksum;
    volatile xdb_manu3_face XDB_FAR *face;
    volatile xdb_manu3_vertex XDB_FAR *vertex_0;
    volatile xdb_manu3_vertex XDB_FAR *vertex_1;
    volatile xdb_manu3_vertex XDB_FAR *vertex_2;
    volatile xdb_u16 XDB_FAR *bucket_heads;
    const volatile xdb_u8 XDB_FAR *raster_bytes;

    if (_dos_allocmem(GEOMETRY_BYTES / 16u, &geometry_segment) != 0u) {
        return write_result("FAIL geometry allocation", 0xffffu);
    }
    if (_dos_allocmem(RASTER_BYTES / 16u, &raster_segment) != 0u) {
        _dos_freemem(geometry_segment);
        return write_result("FAIL raster allocation", 0xffffu);
    }

    fill_segment(geometry_segment, GEOMETRY_BYTES, 0u);
    fill_segment(raster_segment, RASTER_BYTES, INITIAL_BYTE);
    bucket_heads = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_MANU3_BUCKET_HEADS_OFFSET);
    for (offset = 0u; offset < XDB_MANU3_SCREEN_WIDTH; ++offset) {
        bucket_heads[offset] = 0u;
    }
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
    vertex_0->clip_flags = 1u;
    vertex_1->clip_flags = 1u;
    vertex_2->clip_flags = 1u;
    xdb_manu3_face_list_offset = FACE_OFFSET;
    xdb_manu3_face_count = 1u;
    geometry_checksum = checksum_segment(geometry_segment, GEOMETRY_BYTES);

    xdb_manu3_face_bucket_sort(geometry_segment, raster_segment);

    if (checksum_segment(geometry_segment, GEOMETRY_BYTES)
            != geometry_checksum) {
        _dos_freemem(raster_segment);
        _dos_freemem(geometry_segment);
        return write_result("FAIL geometry changed", 0xffffu);
    }

    raster_bytes = XDB_FAR_AT(
            const volatile xdb_u8,
            raster_segment,
            0u);
    for (offset = 0u; offset < RASTER_BYTES; ++offset) {
        xdb_u8 expected = expected_raster_byte(offset);

        if (raster_bytes[offset] != expected) {
            xdb_u8 actual = raster_bytes[offset];

            return write_mismatch(offset, actual, expected);
        }
    }

    _dos_freemem(raster_segment);
    _dos_freemem(geometry_segment);
    return write_result("PASS manu3 renderer empty", 0xffffu);
}
