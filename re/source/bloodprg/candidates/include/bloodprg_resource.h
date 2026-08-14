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
    cb_u16 loaded;
} bloodprg_resource_resolve_result;

typedef struct bloodprg_resource_allocation_result {
    cb_i16 status;
    volatile cb_u8 CB_FAR *destination;
} bloodprg_resource_allocation_result;

typedef struct bloodprg_resource_name_entry {
    char filename[16];
} bloodprg_resource_name_entry;

#pragma pack(1)
typedef struct bloodprg_resource_archive_entry {
    char filename[16];
    cb_u32 byte_count;
    cb_u32 file_offset;
    cb_u8 unknown_18;
} bloodprg_resource_archive_entry;
#pragma pack()

typedef struct bloodprg_resource_descriptor {
    cb_u8 flags;
    cb_u8 variant;
    char filename[1];
} bloodprg_resource_descriptor;

typedef struct bloodprg_resource_index_entry {
    bloodprg_resource_descriptor CB_NEAR *descriptor;
    volatile char CB_NEAR *image_path;
} bloodprg_resource_index_entry;

typedef struct bloodprg_dos_dta {
    cb_u8 reserved_00[0x1a];
    cb_u32 file_size;
} bloodprg_dos_dta;

typedef volatile cb_u8 CB_FAR *bloodprg_resource_buffer_ptr;

#define BLOODPRG_RESOURCE_FLAG_LOADED 0x0003u
#define BLOODPRG_RESOURCE_DIRECT_DESTINATION 0x8000u

extern volatile bloodprg_resource_handle_entry fs_resource_handle_table[]; /* FS:0x0000 */
extern volatile cb_u16 resource_resident_handles[256]; /* FS:0x0800 */
extern volatile cb_u16 resource_eviction_handles[256]; /* FS:0x0A00 */
extern volatile cb_u16 resource_current_handle; /* FS:0x0C00 */
extern volatile cb_u16 CB_FS_DATA
        resource_current_handle_fs; /* explicit FS:0x0C00 alias */
extern volatile cb_u16 resource_current_entry_offset; /* FS:0x0C02 */
extern volatile bloodprg_resource_name_entry CB_FS_DATA
        resource_name_table[]; /* FS:0x0C04 */
extern volatile cb_u32 resource_free_bytes; /* GS:0x0A46 */
extern volatile cb_u32 CB_GAME_DATA
        resource_free_bytes_gs; /* explicit GS:0x0A46 alias */
extern volatile cb_u16 resource_pool_end_segment; /* GS:0x0A6A */
extern volatile cb_u16 CB_GAME_DATA resource_file_header; /* GS:0x0AF2 */
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
extern volatile cb_u32 CB_GAME_DATA
        resource_archive_size;                      /* GS:0x0A52 */
extern volatile cb_u16 CB_GAME_DATA
        resource_archive_handle;                    /* GS:0x0A86 */
extern volatile cb_u16 CB_GAME_DATA
        resource_archive_cache_handle;              /* GS:0x0A88 */
extern const volatile char CB_GAME_DATA
        resource_archive_filename[];                 /* GS:0x00C1 */
extern const volatile char CB_GAME_DATA
        resource_archive_cache_filename[];           /* GS:0x00CB */
extern volatile cb_u32 CB_GAME_DATA
        resource_archive_offset;                    /* GS:0x0A8A */
extern volatile cb_u32 CB_GAME_DATA
        resource_archive_remaining;                 /* GS:0x0A8E */
extern volatile cb_u8 CB_GAME_DATA
        resource_force_write_directory;            /* GS:0x0AE1 */
extern volatile cb_u8 CB_GAME_DATA
        resource_path_is_embedded;                  /* GS:0x0AE2 */
extern bloodprg_resource_buffer_ptr CB_GAME_DATA
        resource_copy_buffer;                       /* GS:0x0A7C */
extern volatile cb_u16 CB_GAME_DATA
        resource_copy_file_handle;                  /* GS:0x0A84 */
extern volatile cb_u8 resource_source_is_banked;    /* game data:0x0DBC */
extern volatile cb_u8 resource_ready_marker;        /* game data:0x0DB7 */
extern const bloodprg_resource_name_entry CB_GAME_DATA
        resource_write_directory_names[];           /* GS:0x0259 */

#if defined(__WATCOMC__)
#pragma aux resource_release parm [ax] modify exact []
#pragma aux resource_free_inner parm [ax] modify exact []
#pragma aux resource_get_field4 parm [ax] value [dx ax] modify exact [ax dx]
#pragma aux lookup_table_1fb5 parm [ax] value [bx] modify [bx]
#pragma aux resource_load_by_id parm [ax] value [ax] modify exact [ax]
#pragma aux resource_named_file_load parm [ax] [es di] value [ax] modify exact [ax]
#pragma aux pbm_image_load_and_decode \
        parm [ds si] [es di] value [ax] modify exact [ax]
#endif

cb_u32 CB_FAR resource_file_load(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *destination); /* 0x01CE:0x07DB */
cb_i16 CB_FAR pbm_image_load_and_decode(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *file_buffer_end); /* 0x01CE:0x091D */
void CB_FAR resource_file_load_to_xms(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *staging_buffer); /* 0x01CE:0x0621 */
void CB_FAR resource_file_load_to_ems(
        volatile char CB_FAR *path); /* 0x01CE:0x0712 */
cb_u32 CB_FAR file_create_and_write(
        const volatile char CB_FAR *path,
        const volatile cb_u8 CB_FAR *source,
        cb_u32 byte_count); /* 0x01CE:0x088B */
void CB_FAR resource_free_inner(cb_u16 handle); /* 0x04B9:0x010C */
void CB_FAR resource_release(cb_u16 handle); /* 0x04B9:0x00F8 */
bloodprg_resource_allocation_result CB_FAR resource_allocate(
        cb_u16 handle, cb_u32 byte_count); /* 0x04B9:0x0000 */
bloodprg_resource_resolve_result CB_FAR resource_handle_resolve(
        cb_u16 handle); /* 0x04B9:0x0190 */
cb_u32 CB_FAR resource_get_field4(cb_u16 handle); /* 0x04B9:0x01AC */
bloodprg_resource_descriptor CB_NEAR *CB_NEAR lookup_table_1fb5(
        cb_u16 index); /* 0x009F80 */
cb_u16 CB_FAR resource_source_select(
        volatile char CB_FAR *filename); /* 0x01CE:0x03B3 */
cb_u16 CB_NEAR resource_archive_match(
        volatile char CB_FAR *filename); /* 0x01CE:0x03EF */
cb_u32 CB_FAR resource_name_lookup(
        volatile char CB_FAR *filename); /* 0x01CE:0x05EA */
/* The binary returns this value in EBP; replacement linking needs an ABI thunk. */
void CB_FAR startup_resource_file_copy(
        volatile char CB_FAR *source_path,
        const volatile char CB_FAR *destination_path); /* 0x01CE:0x052F */
void CB_NEAR resource_archive_index_backing_initialize(void); /* 0x00155F */

int CB_FAR resource_load_by_id(cb_u16 resource_id); /* 0x01CE:0x059B */
int CB_FAR resource_named_file_load(cb_u16 resource_id,
        volatile cb_u8 CB_FAR *direct_destination); /* 0x0299:0x1037 */

volatile bloodprg_dos_dta CB_FAR *CB_NEAR cb_dos_get_dta(void);
int CB_NEAR cb_dos_find_first(const volatile char CB_FAR *path);
/* Open/create publish raw DOS AX through handle on success and failure. */
int CB_NEAR cb_dos_open_read_only(const volatile char CB_FAR *path,
        cb_u16 *handle);
int CB_NEAR cb_dos_create_truncate(const volatile char CB_FAR *path,
        cb_u16 *handle);
int CB_NEAR cb_dos_delete(const volatile char CB_FAR *path);
void CB_NEAR cb_dos_seek_absolute(cb_u16 handle, cb_u32 offset);
cb_u16 CB_NEAR cb_dos_read(cb_u16 handle,
        volatile cb_u8 CB_FAR *destination, cb_u16 byte_count);
void CB_NEAR cb_dos_close(cb_u16 handle);
/* Publishes DOS AX through handle on both success and failure. */
int CB_NEAR cb_dos_create_game_file(
        const volatile char CB_GAME_DATA *path,
        volatile cb_u16 CB_GAME_DATA *handle);
cb_u16 CB_NEAR cb_dos_write(cb_u16 handle,
        const volatile cb_u8 CB_FAR *source, cb_u16 byte_count);
int CB_NEAR resource_switch(cb_u16 resource_id); /* 0x009F8E */
void CB_FAR cb_resource_allocation_failure(cb_u16 error_code);

#endif
