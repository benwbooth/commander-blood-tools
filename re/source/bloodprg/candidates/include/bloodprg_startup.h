#ifndef BLOODPRG_STARTUP_H
#define BLOODPRG_STARTUP_H

#include "bloodprg_common.h"

#define BLOODPRG_COMMAND_TEXT_CAPACITY 127u
#define BLOODPRG_STARTUP_TOKEN_CAPACITY 128u
#define BLOODPRG_WRITE_DIRECTORY_CAPACITY 32u

typedef struct bloodprg_command_tail {
    cb_u8 length;
    char text[BLOODPRG_COMMAND_TEXT_CAPACITY];
} bloodprg_command_tail;

extern char startup_argument_token[BLOODPRG_STARTUP_TOKEN_CAPACITY];
extern cb_u8 startup_write_drive;                   /* game data:0x01B8 */
extern cb_u8 startup_original_drive;                /* game data:0x01B9 */
extern char startup_write_directory[
        BLOODPRG_WRITE_DIRECTORY_CAPACITY];         /* game data:0x01BA */
extern char startup_original_directory[
        BLOODPRG_WRITE_DIRECTORY_CAPACITY];         /* game data:0x01DA */
extern volatile cb_u8 CB_GAME_DATA startup_write_directory_active; /* GS:0x0AE0 */
extern char startup_transient_paths[4][16];          /* game data:0x0DD7 */
extern volatile cb_u8 CB_GAME_DATA startup_audio_driver_id; /* GS:0x0C3B */
extern volatile cb_u16 CB_GAME_DATA startup_audio_configuration; /* GS:0x0C45 */

void CB_NEAR startup_command_line_parse(
        const bloodprg_command_tail CB_FAR *command_tail); /* 0x0006F1 */
void CB_NEAR startup_option_apply(char *token);             /* 0x000726 */
void CB_NEAR startup_transient_files_delete(void);          /* 0x00147F */
void CB_FAR startup_write_directory_enter(void);            /* 0x0027C3 */
void CB_FAR startup_original_directory_restore(void);       /* 0x0027E9 */

#endif
