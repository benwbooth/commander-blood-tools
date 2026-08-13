#ifndef BLOODPRG_PLATFORM_H
#define BLOODPRG_PLATFORM_H

#include "bloodprg_common.h"

#define BLOODPRG_MSCDEX_INTERRUPT 0x2fu
#define BLOODPRG_MSCDEX_SEND_DEVICE_REQUEST 0x1510u
#define BLOODPRG_MSCDEX_IOCTL_INPUT 0x03u
#define BLOODPRG_MSCDEX_IOCTL_OUTPUT 0x0cu
#define BLOODPRG_MSCDEX_PLAY_AUDIO 0x84u
#define BLOODPRG_MSCDEX_STOP_AUDIO 0x85u

#pragma pack(1)
typedef struct bloodprg_mscdex_request_header {
    cb_u8 length;
    cb_u8 subunit;
    cb_u8 command;
    cb_u16 status;
    cb_u8 reserved[8];
} bloodprg_mscdex_request_header;

typedef struct bloodprg_mscdex_ioctl_request {
    bloodprg_mscdex_request_header header;
    cb_u8 media_descriptor;
    cb_u16 transfer_offset;
    cb_u16 transfer_segment;
    cb_u16 transfer_count;
    cb_u8 untouched_tail[6];
} bloodprg_mscdex_ioctl_request;

typedef struct bloodprg_cdrom_disc_info {
    cb_u8 function;
    cb_u8 first_track;
    cb_u8 last_track;
    cb_u32 lead_out_position;
} bloodprg_cdrom_disc_info;

typedef struct bloodprg_cdrom_channel_control {
    cb_u8 function;
    cb_u8 input_channel_0;
    cb_u8 volume_0;
    cb_u8 input_channel_1;
    cb_u8 volume_1;
    cb_u8 input_channel_2;
    cb_u8 volume_2;
    cb_u8 input_channel_3;
    cb_u8 volume_3;
} bloodprg_cdrom_channel_control;

typedef struct bloodprg_cdrom_track_info {
    cb_u8 function;
    cb_u8 track_number;
    cb_u32 start_position;
    cb_u8 control;
} bloodprg_cdrom_track_info;

typedef struct bloodprg_mscdex_audio_request {
    bloodprg_mscdex_request_header header;
    cb_u8 address_mode;
    cb_u32 start_position;
    cb_u32 sector_count;
} bloodprg_mscdex_audio_request;
#pragma pack()

typedef char bloodprg_mscdex_header_size_must_be_13[
        sizeof(bloodprg_mscdex_request_header) == 13 ? 1 : -1];
typedef char bloodprg_mscdex_ioctl_size_must_be_26[
        sizeof(bloodprg_mscdex_ioctl_request) == 26 ? 1 : -1];
typedef char bloodprg_cdrom_disc_info_size_must_be_7[
        sizeof(bloodprg_cdrom_disc_info) == 7 ? 1 : -1];
typedef char bloodprg_cdrom_channel_control_size_must_be_9[
        sizeof(bloodprg_cdrom_channel_control) == 9 ? 1 : -1];
typedef char bloodprg_cdrom_track_info_size_must_be_7[
        sizeof(bloodprg_cdrom_track_info) == 7 ? 1 : -1];
typedef char bloodprg_mscdex_audio_size_must_be_22[
        sizeof(bloodprg_mscdex_audio_request) == 22 ? 1 : -1];

extern volatile cb_i16 rtc_hour;        /* GS:0x0AA6 */
extern volatile cb_i16 rtc_day;         /* GS:0x0AA8 */
extern volatile cb_i16 rtc_month;       /* GS:0x0AAA */
extern volatile cb_i16 rtc_year;        /* GS:0x0AAC */
extern volatile cb_u8 cdrom_present;    /* GS:0x0AE6 */
extern volatile bloodprg_mscdex_ioctl_request CB_GAME_DATA
        cdrom_ioctl_request;            /* GS:0x0B41 */
extern volatile bloodprg_cdrom_disc_info CB_GAME_DATA
        cdrom_disc_info;                /* GS:0x0B5B */
extern volatile bloodprg_cdrom_channel_control CB_GAME_DATA
        cdrom_channel_control;          /* GS:0x0B62 */
extern volatile bloodprg_cdrom_track_info CB_GAME_DATA
        cdrom_track_info;               /* GS:0x0B6B */
extern volatile bloodprg_mscdex_audio_request CB_GAME_DATA
        cdrom_audio_request;            /* GS:0x0B72 */

void CB_FAR rtc_time_read(void);        /* 0x00093B */
void CB_FAR rtc_date_read(void);        /* 0x000950 */
void CB_NEAR detect_cdrom(void);        /* 0x000B32 */
void CB_NEAR cdrom_audio_prepare(void); /* 0x001344 */
void CB_FAR cdrom_audio_stop(void);     /* 0x001397 */
void CB_FAR cdrom_audio_play_track_2(void); /* 0x0013C4 */
void CB_FAR mouse_set_ranges(cb_u16 min_x, cb_u16 max_x,
        cb_u16 min_y, cb_u16 max_y);    /* 0x000D4A */
void CB_FAR mouse_reset_hide(void);      /* 0x000CEF */
void CB_FAR print_string_dos(
        const volatile char *text);     /* 0x000D61 */
cb_u16 CB_FAR kbd_read_int16(void);     /* 0x00267D */

#if defined(__WATCOMC__)
#pragma aux mouse_set_ranges parm [ax] [bx] [cx] [dx]
#pragma aux print_string_dos parm [si]
#pragma aux cdrom_audio_prepare modify exact []
#pragma aux cdrom_audio_stop modify exact []
#pragma aux cdrom_audio_play_track_2 modify exact []
#endif

#endif
