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
extern cb_u8 startup_current_drive;                 /* SS:0x01B9 */
extern char startup_write_directory[
        BLOODPRG_WRITE_DIRECTORY_CAPACITY];         /* SS:0x01BA */
extern volatile cb_u8 CB_GAME_DATA startup_audio_driver_id; /* GS:0x0C3B */
extern volatile cb_u16 CB_GAME_DATA startup_audio_configuration; /* GS:0x0C45 */

void CB_NEAR startup_command_line_parse(
        const bloodprg_command_tail CB_FAR *command_tail); /* 0x0006F1 */
void CB_NEAR startup_option_apply(char *token);             /* 0x000726 */

#endif
