#ifndef BLOODPRG_RESOURCE_H
#define BLOODPRG_RESOURCE_H

#include "bloodprg_common.h"

typedef struct bloodprg_resource_handle_entry {
    cb_u16 unknown_00;
    cb_u16 unknown_02;
    cb_u32 field_04;
} bloodprg_resource_handle_entry;

typedef struct bloodprg_resource_resolve_result {
    cb_u16 segment;
    cb_u16 offset;
    int loaded;
} bloodprg_resource_resolve_result;

typedef struct bloodprg_resource_descriptor {
    cb_u8 flags;
    cb_u8 variant;
    char filename[1];
} bloodprg_resource_descriptor;

typedef struct bloodprg_resource_index_entry {
    bloodprg_resource_descriptor *descriptor;
    cb_u16 secondary_resource_id;
} bloodprg_resource_index_entry;

typedef struct bloodprg_dos_dta {
    cb_u8 reserved_00[0x1a];
    cb_u32 file_size;
} bloodprg_dos_dta;

#define BLOODPRG_RESOURCE_FLAG_LOADED 0x0003u

extern const volatile bloodprg_resource_handle_entry fs_resource_handle_table[]; /* FS:0x0000 */
extern volatile bloodprg_resource_index_entry resource_index[]; /* DS:0x1FB5 */
extern volatile cb_u8 resource_variant;             /* game data:0x1FB1 */
extern volatile cb_u16 resource_requested_id;       /* game data:0x0D80 */
extern volatile cb_u16 resource_active_id;          /* game data:0x0D82 */
extern volatile cb_u16 resource_flags;              /* game data:0x0D76 */
extern volatile cb_u32 resource_range_start;        /* game data:0x0D6E */
extern volatile cb_u32 resource_range_remaining;    /* game data:0x0D72 */
extern volatile cb_u32 resource_index_start;        /* game data:0x0D78 */
extern volatile cb_u32 resource_index_remaining;    /* game data:0x0D7C */
extern volatile cb_u32 resource_source_offset;      /* game data:0x0D84 */
extern volatile cb_u32 resource_source_remaining;   /* game data:0x0D88 */
extern volatile cb_u32 resource_archive_size;       /* game data:0x0A52 */
extern volatile cb_u32 resource_archive_offset;     /* game data:0x0A8A */
extern volatile cb_u32 resource_archive_remaining;  /* game data:0x0A8E */
extern volatile cb_u8 resource_path_is_embedded;    /* game data:0x0AE2 */
extern volatile cb_u8 resource_source_is_banked;    /* game data:0x0DBC */
extern volatile cb_u8 resource_ready_marker;        /* game data:0x0DB7 */
extern volatile char resource_path_buffer[];        /* game data:0x0259 */

#if defined(__WATCOMC__)
#pragma aux lookup_table_1fb5 parm [ax] value [bx] modify [bx]
#pragma aux path_builder_gs_relative parm [dx] value [bx] modify [bx cx dx]
#endif

cb_u32 CB_FAR resource_file_load(const volatile char *path,
        volatile cb_u8 CB_FAR *destination); /* 0x01CE:0x07DB */
void CB_FAR resource_free_inner(cb_u16 handle); /* 0x04B9:0x010C */
bloodprg_resource_descriptor *CB_NEAR lookup_table_1fb5(
        cb_u16 index); /* 0x009F80 */
cb_u16 CB_FAR path_builder_gs_relative(
        const volatile char *filename); /* 0x01CE:0x03B3 */

volatile bloodprg_dos_dta CB_FAR *CB_NEAR cb_dos_get_dta(void);
void CB_NEAR cb_dos_find_first(const volatile char *path);
int CB_NEAR cb_dos_open_read_only(const volatile char *path,
        cb_u16 *handle);
int CB_NEAR resource_switch(cb_u16 resource_id); /* 0x009F8E */

#endif
