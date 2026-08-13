#include <dos.h>
#include <stdio.h>

#include "xdb_manu3.h"

#define GEOMETRY_BYTES 0x4000u
#define RASTER_BYTES 0x3000u
#define FACE_OFFSET 0x1000u
#define VERTEX_0_OFFSET 0x1100u
#define VERTEX_1_OFFSET 0x1200u
#define VERTEX_2_OFFSET 0x1300u
#define RECORD_OFFSET 0x2000u
#define FREE_OFFSET 0x205au
#define RESULT_FILE "RESULT.TXT"
#define RECORD_FILE "RECORD.BIN"

volatile xdb_manu3_segment_directory xdb_manu3_segments;

static void fill_pattern(
        xdb_u16 segment,
        xdb_u16 size,
        xdb_u8 multiplier,
        xdb_u8 addend)
{
    volatile xdb_u8 XDB_FAR *bytes = XDB_FAR_AT(
            volatile xdb_u8,
            segment,
            0u);
    xdb_u16 offset;

    for (offset = 0u; offset < size; ++offset) {
        bytes[offset] = (xdb_u8)(offset * multiplier + addend);
    }
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

static int write_record(xdb_u16 raster_segment)
{
    const volatile xdb_u8 XDB_FAR *record = XDB_FAR_AT(
            const volatile xdb_u8,
            raster_segment,
            RECORD_OFFSET);
    FILE *output = fopen(RECORD_FILE, "wb");
    xdb_u16 offset;

    if (output == NULL) {
        return 0;
    }
    for (offset = 0u; offset < sizeof(xdb_manu3_raster_record); ++offset) {
        if (fputc(record[offset], output) == EOF) {
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
    volatile xdb_manu3_face XDB_FAR *face;
    volatile xdb_manu3_vertex XDB_FAR *vertex_0;
    volatile xdb_manu3_vertex XDB_FAR *vertex_1;
    volatile xdb_manu3_vertex XDB_FAR *vertex_2;
    volatile xdb_i32 XDB_FAR *reciprocal_table;
    volatile xdb_manu3_raster_record XDB_FAR *record;
    volatile xdb_manu3_raster_record XDB_FAR *head;
    volatile xdb_manu3_raster_record XDB_FAR *tail;
    volatile xdb_u16 XDB_FAR *free_head;

    if (_dos_allocmem(GEOMETRY_BYTES / 16u, &geometry_segment) != 0u) {
        return write_result("FAIL geometry allocation");
    }
    if (_dos_allocmem(RASTER_BYTES / 16u, &raster_segment) != 0u) {
        return write_result("FAIL raster allocation");
    }

    fill_pattern(geometry_segment, GEOMETRY_BYTES, 13u, 126u);
    fill_pattern(raster_segment, RASTER_BYTES, 37u, 82u);
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
    reciprocal_table = XDB_FAR_AT(
            volatile xdb_i32,
            raster_segment,
            0u);
    record = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            RECORD_OFFSET);
    head = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET);
    tail = XDB_FAR_AT(
            volatile xdb_manu3_raster_record,
            raster_segment,
            XDB_MANU3_ACTIVE_LIST_TAIL_OFFSET);
    free_head = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            0x0908u);

    reciprocal_table[80] = 0x00000333l;
    reciprocal_table[90] = 0x000002d8l;
    face->link = 0u;
    face->vertex_0 = VERTEX_0_OFFSET;
    face->vertex_1 = VERTEX_1_OFFSET;
    face->vertex_2 = VERTEX_2_OFFSET;

    vertex_0->link = 0x0123u;
    vertex_0->field_002 = 0x1234u;
    vertex_0->screen.position.x = 10;
    vertex_0->screen.position.y = 20;
    vertex_0->depth = 0x10203040l;
    vertex_1->link = 0x2345u;
    vertex_1->field_002 = 0x3456u;
    vertex_1->screen.position.x = 10;
    vertex_1->screen.position.y = 100;
    vertex_1->depth = -0x01234567l;
    vertex_2->link = 0x4567u;
    vertex_2->field_002 = 0x5678u;
    vertex_2->screen.position.x = 100;
    vertex_2->screen.position.y = 30;
    vertex_2->depth = 0x30405060l;

    *free_head = RECORD_OFFSET;
    record->next = FREE_OFFSET;
    head->next = XDB_MANU3_ACTIVE_LIST_TAIL_OFFSET;
    tail->edge_0_position = 0x7fffffffl;
    tail->edge_0_step = 0x7fffffffl;
    tail->previous = XDB_MANU3_ACTIVE_LIST_HEAD_OFFSET;
    xdb_manu3_segments.work_segment_1 = 0x3210u;

    xdb_manu3_face_activate(face, raster_segment);

    if (*free_head != FREE_OFFSET
            || head->next != RECORD_OFFSET
            || tail->previous != RECORD_OFFSET) {
        return write_result("FAIL active list state");
    }
    if (!write_record(raster_segment)) {
        return write_result("FAIL record write");
    }

    _dos_freemem(raster_segment);
    _dos_freemem(geometry_segment);
    return write_result("PASS manu3 face activate");
}
