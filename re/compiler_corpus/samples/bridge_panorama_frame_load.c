/* Codegen probe for BLOODPRG 0x00981B. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <string.h>
#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define FAR
#define NEAR
#define GAME_DATA FAR
void FAR *NEAR _fmemcpy(void FAR *destination,
        const void FAR *source, u16 count);
#endif

typedef struct directory_entry {
    u32 file_offset;
    u32 byte_count;
} directory_entry;

typedef struct station_record {
    u8 prefix[12];
    i16 orb_box[4];
    u8 suffix[4];
} station_record;

extern volatile u16 file_handle;
extern volatile directory_entry directory;
extern volatile u8 FAR *load_buffer;
extern volatile station_record GAME_DATA stations[];
extern volatile u8 GAME_DATA palette_refresh;
extern volatile u8 GAME_DATA live_palette[768];
extern volatile u8 GAME_DATA panorama_palette[768];

void NEAR seek_probe(u16 handle, u32 offset);
u16 NEAR read_probe(u16 handle, volatile u8 FAR *destination, u16 byte_count);
void FAR unpack_probe(const u8 FAR *source);

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemcpy)
#pragma aux bridge_panorama_frame_load_probe parm [ax] modify exact []
#endif

void NEAR bridge_panorama_frame_load_probe(u16 frame)
{
    volatile station_record GAME_DATA *station;
    volatile u8 FAR *chunk;
    u16 directory_offset;
    u16 station_index;
    u16 handle;
    u16 index;

    directory_offset = (u16)(frame << 3);
    handle = file_handle;
    seek_probe(handle, (u32)directory_offset);
    (void)read_probe(handle, (volatile u8 FAR *)&directory, 8u);

    seek_probe(handle, directory.file_offset);
    chunk = load_buffer;
    (void)read_probe(handle, chunk, (u16)directory.byte_count);

    for (index = 0; index < 4u; ++index) {
        stations[index].orb_box[0] = -1;
        stations[index].orb_box[1] = -1;
        stations[index].orb_box[2] = -1;
        stations[index].orb_box[3] = -1;
    }

    station_index = *(const volatile u16 FAR *)(chunk + 8u);
    station = &stations[station_index];
    _fmemcpy((void FAR *)station->orb_box, (const void FAR *)chunk, 8u);
    unpack_probe((const u8 FAR *)(chunk + 10u));

    if ((palette_refresh & 1u) != 0) {
        _fmemcpy((void FAR *)live_palette,
                (const void FAR *)panorama_palette, 768u);
    }
}
