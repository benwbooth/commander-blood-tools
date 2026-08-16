#include <stdio.h>
#include <string.h>

#include "bloodprg_startup.h"

#define RESULT_FILE "RESULT.TXT"

/* Canonical storage for the startup parser slice. */
char startup_argument_token[BLOODPRG_STARTUP_TOKEN_CAPACITY];
cb_u8 startup_write_drive;
cb_u8 startup_original_drive;
char startup_write_directory[BLOODPRG_WRITE_DIRECTORY_CAPACITY];
volatile cb_u8 CB_GAME_DATA startup_audio_driver_id;
volatile cb_u16 CB_GAME_DATA startup_audio_configuration;

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

static void reset_state(void)
{
    memset(startup_argument_token, 0, sizeof(startup_argument_token));
    memset(startup_write_directory, 0, sizeof(startup_write_directory));
    startup_write_drive = 0u;
    startup_original_drive = 0u;
    startup_audio_driver_id = 0u;
    startup_audio_configuration = 0u;
}

int main(void)
{
    static const char command_text[] = "S161234 WRIC:\\WRITE\\";
    bloodprg_command_tail command_tail;

    reset_state();
    command_tail.length = (cb_u8)(sizeof(command_text) - 1u);
    memcpy(command_tail.text, command_text, command_tail.length);
    startup_command_line_parse(&command_tail);

    if (startup_audio_driver_id != 0x2au
            || startup_audio_configuration != 0x07b4u) {
        return write_result("FAIL audio option state");
    }
    if (strcmp(startup_write_directory, "C:\\WRITE") != 0) {
        return write_result("FAIL write directory state");
    }
    if (strcmp(startup_argument_token, "WRIC:\\WRITE\\") != 0) {
        return write_result("FAIL final token state");
    }
    return write_result("PASS bloodprg startup options");
}
