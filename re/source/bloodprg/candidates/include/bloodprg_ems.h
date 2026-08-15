#ifndef BLOODPRG_EMS_H
#define BLOODPRG_EMS_H

#include "bloodprg_common.h"

typedef union bloodprg_xms_address {
    cb_u32 offset;
    volatile cb_u8 CB_FAR *pointer;
} bloodprg_xms_address;

typedef struct bloodprg_xms_move_request {
    cb_u32 length;
    cb_u16 source_handle;
    bloodprg_xms_address source;
    cb_u16 destination_handle;
    bloodprg_xms_address destination;
} bloodprg_xms_move_request;

typedef union bloodprg_snd_storage_cursor {
    cb_u32 xms_offset;
    struct {
        cb_u16 logical_page;
        cb_u16 reserved;
    } ems;
} bloodprg_snd_storage_cursor;

typedef void (CB_FAR *bloodprg_xms_driver_entry)(void);

/* Raw callers establish the game-data segment before the XMS dispatch. */
extern volatile cb_u8 snd_bank_storage_mode; /* DS=GS:0x0B9F */
extern volatile cb_u8 CB_GAME_DATA
        snd_bank_storage_mode_gs; /* explicit GS:0x0B9F alias */
extern bloodprg_xms_driver_entry CB_GAME_DATA xms_driver_entry; /* GS:0x0A4A */
extern volatile cb_i16 CB_GAME_DATA resource_xms_handle; /* GS:0x0A56 */
extern volatile cb_i16 CB_GAME_DATA resource_ems_handle; /* GS:0x0A58 */
extern volatile cb_i16 CB_GAME_DATA secondary_xms_handle; /* GS:0x0A5A */
extern volatile cb_i16 CB_GAME_DATA secondary_ems_handle; /* GS:0x0A5C */
extern volatile cb_i16 CB_GAME_DATA snd_bank_xms_handle; /* GS:0x0A5E */
extern volatile cb_i16 CB_GAME_DATA snd_bank_ems_handle; /* GS:0x0A60 */
extern volatile cb_i16 CB_GAME_DATA small_xms_handle; /* GS:0x0A62 */
extern volatile cb_i16 CB_GAME_DATA small_ems_handle; /* GS:0x0A64 */
extern volatile cb_u16 CB_GAME_DATA ems_page_frame_segment; /* GS:0x0A66 */
extern volatile bloodprg_snd_storage_cursor CB_GAME_DATA
        snd_storage_cursor; /* GS:0x0A4E */
extern volatile bloodprg_xms_move_request CB_GAME_DATA
        xms_move_request; /* GS:0x0A6C */
extern volatile cb_u8 CB_FAR ems_page_frame[]; /* segment at GS:0x0A66 */
extern volatile cb_u16 CB_GAME_DATA snd_voice_file_handle; /* GS:0x0C47 */
extern volatile cb_u16 CB_GAME_DATA snd_bank_file_handle; /* GS:0x0C49 */

void CB_NEAR cb_ems_map_page(cb_u16 handle, cb_u16 logical_page,
        cb_u8 physical_page);
void CB_NEAR cb_xms_move(
        volatile bloodprg_xms_move_request CB_GAME_DATA *request);
int CB_NEAR cb_xms_allocate_kb(cb_u16 kilobytes, cb_u16 *handle);
void CB_NEAR cb_xms_release(cb_u16 handle);
void CB_FAR far_memmove(volatile cb_u8 CB_FAR *destination,
        const volatile cb_u8 CB_FAR *source, cb_u32 byte_count); /* 01CE:0B93 */

void CB_NEAR snd_bank_ems_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD26 */
void CB_NEAR snd_bank_xms_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD4E */
void CB_NEAR snd_bank_file_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD8D */
void CB_NEAR snd_bank_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD09 */
void CB_FAR extended_memory_backends_init(void); /* 0x00099F */
void CB_FAR extended_memory_backends_release(void); /* 0x000A99 */

#if defined(__WATCOMC__)
#pragma aux snd_bank_ems_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_xms_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_file_page_read parm [ax] [es di] modify exact []
#pragma aux snd_bank_page_read parm [ax] [es di]
#endif

#endif
