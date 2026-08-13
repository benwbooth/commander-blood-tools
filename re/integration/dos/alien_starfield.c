#include <dos.h>
#include <stdio.h>

#include "xdb_alien.h"

#define SEGMENT_PARAGRAPHS 0x1000u
#define RESULT_FILE "RESULT.TXT"
#define STATE_FILE "STATE.BIN"

volatile xdb_u16 xdb_alien_raster_segment;
volatile xdb_u16 xdb_alien_framebuffer_segment;
volatile xdb_i32 xdb_alien_camera_matrix[9] = {
    0l,
    0x00000100l,
    0l,
    0l,
    0l,
    0x00000100l,
    0x00010000l,
    0l,
    0l,
};
volatile xdb_i32 xdb_alien_camera_position[3] = {0l, 0l, 0l};

static void fill_full_segment(
        xdb_u16 segment,
        xdb_u8 multiplier,
        xdb_u8 addend)
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

static int write_segment(FILE *output, xdb_u16 segment)
{
    const volatile xdb_u8 XDB_FAR *bytes = XDB_FAR_AT(
            const volatile xdb_u8,
            segment,
            0u);
    xdb_u16 offset = 0u;

    do {
        if (fputc(bytes[offset], output) == EOF) {
            return 0;
        }
        ++offset;
    } while (offset != 0u);
    return 1;
}

static int write_state(xdb_u16 raster_segment, xdb_u16 framebuffer_segment)
{
    FILE *output = fopen(STATE_FILE, "wb");
    int result;

    if (output == NULL) {
        return 0;
    }
    result = write_segment(output, raster_segment)
            && write_segment(output, framebuffer_segment);
    if (fclose(output) != 0) {
        result = 0;
    }
    return result;
}

int main(void)
{
    xdb_u16 raster_segment;
    xdb_u16 framebuffer_segment;
    xdb_u16 shade;
    volatile xdb_u8 XDB_FAR *shade_table;
    volatile xdb_u32 XDB_FAR *seed;
    volatile xdb_i16 XDB_FAR *remaining;
    volatile xdb_u16 XDB_FAR *cursors;

    if (_dos_allocmem(SEGMENT_PARAGRAPHS, &raster_segment) != 0u) {
        return write_result("FAIL raster allocation");
    }
    if (_dos_allocmem(SEGMENT_PARAGRAPHS, &framebuffer_segment) != 0u) {
        _dos_freemem(raster_segment);
        return write_result("FAIL framebuffer allocation");
    }

    fill_full_segment(raster_segment, 7u, 36u);
    fill_full_segment(framebuffer_segment, 11u, 20u);
    shade_table = XDB_FAR_AT(
            volatile xdb_u8,
            raster_segment,
            XDB_CROOLIS_STAR_SHADE_TABLE_OFFSET);
    for (shade = 0u; shade != 256u; ++shade) {
        shade_table[shade] = (xdb_u8)(shade * 37u + 54u);
    }
    seed = XDB_FAR_AT(
            volatile xdb_u32,
            raster_segment,
            XDB_CROOLIS_STAR_SEED_OFFSET);
    *seed = 0x12345678ul;
    xdb_alien_raster_segment = raster_segment;
    xdb_alien_framebuffer_segment = framebuffer_segment;

    xdb_croolis_render_starfield();

    remaining = XDB_FAR_AT(
            volatile xdb_i16,
            raster_segment,
            XDB_CROOLIS_STAR_REMAINING_OFFSET);
    cursors = XDB_FAR_AT(
            volatile xdb_u16,
            raster_segment,
            XDB_CROOLIS_STAR_CURSORS_OFFSET);
    if (*remaining != -1
            || cursors[0] != 0x243au
            || cursors[1] != 0x26bau
            || cursors[2] != 0x2c22u
            || cursors[3] != 0x32eeu) {
        return write_result("FAIL starfield state");
    }
    if (!write_state(raster_segment, framebuffer_segment)) {
        return write_result("FAIL state write");
    }

    _dos_freemem(framebuffer_segment);
    _dos_freemem(raster_segment);
    return write_result("PASS alien starfield");
}
