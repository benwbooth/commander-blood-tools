#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"

#define BRIDGE_PANORAMA_DIRECTORY_ENTRY_BYTES 8u
#define BRIDGE_PANORAMA_HEADER_BYTES 10u

void CB_NEAR bridge_panorama_frame_load(cb_u16 frame)
{
    volatile bridge_panorama_station_record CB_GAME_DATA *station;
    volatile cb_u8 CB_FAR *chunk;
    cb_u16 directory_offset;
    cb_u16 station_index;
    cb_u16 handle;
    cb_u16 index;

    directory_offset = (cb_u16)(frame << 3);
    handle = bridge_panorama_file_handle;
    cb_dos_seek_absolute(handle, (cb_u32)directory_offset);
    (void)cb_dos_read(handle,
            (volatile cb_u8 CB_FAR *)&bridge_panorama_directory,
            BRIDGE_PANORAMA_DIRECTORY_ENTRY_BYTES);

    cb_dos_seek_absolute(handle, bridge_panorama_directory.file_offset);
    chunk = bridge_panorama_load_buffer;
    (void)cb_dos_read(handle, chunk,
            (cb_u16)bridge_panorama_directory.byte_count);

    for (index = 0; index < BRIDGE_PANORAMA_STATION_COUNT; ++index) {
        bridge_panorama_stations[index].orb_box[0] = -1;
        bridge_panorama_stations[index].orb_box[1] = -1;
        bridge_panorama_stations[index].orb_box[2] = -1;
        bridge_panorama_stations[index].orb_box[3] = -1;
    }

    station_index = *(const volatile cb_u16 CB_FAR *)(chunk + 8u);
    station = &bridge_panorama_stations[station_index];
    _fmemcpy((void CB_FAR *)station->orb_box,
            (const void CB_FAR *)chunk, 8u);
    bridge_panorama_frame_unpack(
            (const cb_u8 CB_FAR *)(chunk + BRIDGE_PANORAMA_HEADER_BYTES));

    if ((pbm_palette_refresh & 1u) != 0) {
        _fmemcpy((void CB_FAR *)pbm_live_palette,
                (const void CB_FAR *)bridge_panorama_palette,
                768u);
    }
}
