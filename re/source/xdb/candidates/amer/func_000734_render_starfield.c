#include "../include/xdb_alien.h"
#include "../include/xdb_video.h"

void XDB_NEAR xdb_amer_render_starfield(void)
{
    xdb_u16 raster_segment = xdb_alien_raster_segment;
    volatile xdb_u8 XDB_FAR *shade_table =
            XDB_FAR_AT(
                    volatile xdb_u8,
                    raster_segment,
                    XDB_AMER_STAR_SHADE_TABLE_OFFSET);
    volatile xdb_u32 XDB_FAR *seed =
            XDB_FAR_AT(
                    volatile xdb_u32,
                    raster_segment,
                    XDB_AMER_STAR_SEED_OFFSET);
    volatile xdb_i16 XDB_FAR *remaining =
            XDB_FAR_AT(
                    volatile xdb_i16,
                    raster_segment,
                    XDB_AMER_STAR_REMAINING_OFFSET);
    volatile xdb_u16 XDB_FAR *plane_cursors =
            XDB_FAR_AT(
                    volatile xdb_u16,
                    raster_segment,
                    XDB_AMER_STAR_CURSORS_OFFSET);
    volatile xdb_i32 XDB_FAR *matrix =
            XDB_FAR_AT(
                    volatile xdb_i32,
                    raster_segment,
                    XDB_AMER_STAR_MATRIX_OFFSET);
    volatile xdb_alien_star_camera_cell XDB_FAR *camera_cells =
            XDB_FAR_AT(
                    volatile xdb_alien_star_camera_cell,
                    raster_segment,
                    XDB_AMER_STAR_CAMERA_CELLS_OFFSET);
    volatile xdb_u8 XDB_FAR *framebuffer;
    xdb_u32 random;
    xdb_u16 index;

    for (index = 0u; index != 9u; ++index) {
        matrix[index] = xdb_alien_camera_matrix[index];
    }
    camera_cells[0].coordinate =
            (xdb_u16)((xdb_u32)xdb_alien_camera_position[0] >> 13);
    camera_cells[1].coordinate =
            (xdb_u16)((xdb_u32)xdb_alien_camera_position[1] >> 13);
    camera_cells[2].coordinate =
            (xdb_u16)((xdb_u32)xdb_alien_camera_position[2] >> 13);

    *remaining = (xdb_i16)(XDB_ALIEN_STAR_COUNT - 1u);
    for (index = 0u; index != 4u; ++index) {
        plane_cursors[index] = (xdb_u16)(
                XDB_AMER_STAR_RECORDS_OFFSET
                + index * XDB_ALIEN_STAR_PLANE_STRIDE);
    }

    random = *seed;
    do {
        xdb_i32 position_x;
        xdb_i32 position_y;
        xdb_i32 position_z;
        xdb_u32 accumulator;
        xdb_i32 depth;

        random = (random >> 7) | (random << 25);
        random -= random >> 31;
        position_x = (xdb_i32)(xdb_i16)(
                camera_cells[0].coordinate - (xdb_u16)random);
        random = (random >> 7) | (random << 25);
        random -= random >> 31;
        position_y = (xdb_i32)(xdb_i16)(
                camera_cells[1].coordinate - (xdb_u16)random);
        random = (random >> 7) | (random << 25);
        random -= random >> 31;
        position_z = (xdb_i32)(xdb_i16)(
                camera_cells[2].coordinate - (xdb_u16)random);

        accumulator = (xdb_u32)matrix[6] * (xdb_u32)position_x;
        accumulator += (xdb_u32)matrix[7] * (xdb_u32)position_y;
        accumulator += (xdb_u32)matrix[8] * (xdb_u32)position_z;
        depth = (xdb_i32)accumulator;
        if (depth >= 0) {
            depth >>= 8;
            if (depth != 0) {
                xdb_u32 screen_y_accumulator =
                        (xdb_u32)matrix[3] * (xdb_u32)position_x;
                xdb_u32 screen_x_accumulator =
                        (xdb_u32)matrix[0] * (xdb_u32)position_x;
                xdb_i16 screen_x;
                xdb_i16 screen_y;

                screen_y_accumulator +=
                        (xdb_u32)matrix[4] * (xdb_u32)position_y;
                screen_y_accumulator +=
                        (xdb_u32)matrix[5] * (xdb_u32)position_z;
                screen_x_accumulator +=
                        (xdb_u32)matrix[1] * (xdb_u32)position_y;
                screen_x_accumulator +=
                        (xdb_u32)matrix[2] * (xdb_u32)position_z;

                screen_x = (xdb_i16)(
                        (xdb_i32)screen_x_accumulator / depth);
                screen_x = (xdb_i16)((xdb_u16)screen_x + 160u);
                if (screen_x >= 0 && screen_x < XDB_ALIEN_SCREEN_WIDTH) {
                    screen_y = (xdb_i16)(
                            (xdb_i32)screen_y_accumulator / depth);
                    screen_y = (xdb_i16)(
                            (xdb_u16)(0u - (xdb_u16)screen_y) + 100u);
                    if (screen_y >= 0
                            && screen_y < XDB_ALIEN_SCREEN_HEIGHT) {
                        xdb_u16 plane = (xdb_u16)screen_x & 3u;
                        xdb_u16 record_offset = plane_cursors[plane];
                        volatile xdb_alien_star_record XDB_FAR *record =
                                XDB_FAR_AT(
                                        volatile xdb_alien_star_record,
                                        raster_segment,
                                        record_offset);

                        plane_cursors[plane] = (xdb_u16)(
                                record_offset
                                + sizeof(xdb_alien_star_record));
                        record->framebuffer_offset = (xdb_u16)(
                                ((xdb_u16)screen_y * XDB_ALIEN_SCREEN_WIDTH
                                 + (xdb_u16)screen_x)
                                >> 2);
                        record->shade = (xdb_u16)((xdb_u32)depth >> 15);
                    }
                }
            }
        }
    } while (--*remaining >= 0);

    framebuffer = XDB_FAR_AT(
            volatile xdb_u8,
            xdb_alien_framebuffer_segment,
            0u);
    for (index = 0u; index != 4u; ++index) {
        xdb_u16 record_offset = (xdb_u16)(
                XDB_AMER_STAR_RECORDS_OFFSET
                + index * XDB_ALIEN_STAR_PLANE_STRIDE);
        xdb_u16 end_offset = plane_cursors[index];

        if (record_offset < end_offset) {
            xdb_port_write_u16(
                    0x03c4u,
                    (xdb_u16)(((index + 1u) << 8) | 0x0002u));
            do {
                volatile xdb_alien_star_record XDB_FAR *record =
                        XDB_FAR_AT(
                                volatile xdb_alien_star_record,
                                raster_segment,
                                record_offset);

                framebuffer[record->framebuffer_offset] =
                        shade_table[record->shade];
                record_offset = (xdb_u16)(
                        record_offset + sizeof(xdb_alien_star_record));
            } while (record_offset < end_offset);
        }
    }
}
