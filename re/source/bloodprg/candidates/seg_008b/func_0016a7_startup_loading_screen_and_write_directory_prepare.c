#include <direct.h>
#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_hardware.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

#define STARTUP_COPY_FILE_ATTRIBUTES 0x18u

void CB_NEAR startup_loading_screen_and_write_directory_prepare(void)
{
    bloodprg_graphics_buffer_ptr volatile saved_draw_framebuffer;
    const bloodprg_resource_name_entry CB_GAME_DATA *entry;
    struct find_t find_data;
    union REGS registers;
    struct SREGS segments;
    char *source_append;
    char *destination_append;
    const char *source;
    char *destination;
    char character;
    unsigned current_drive;

    vga_palette_write(bridge_panorama_palette);
    blit_fill_row_5221(0u);
    (void)font8x8_text_draw_display(
            startup_loading_text, 130u, 96u, 0xffefu);

    saved_draw_framebuffer = graphics_draw_framebuffer;
    graphics_draw_framebuffer = graphics_screen_buffer;
    chunky_to_planar_framebuffer(graphics_display_buffer);
    graphics_draw_framebuffer = saved_draw_framebuffer;

    (void)mkdir(startup_write_directory);
    startup_write_drive = (cb_u8)(startup_write_directory[0] - 'A');

    _dos_getdrive(&current_drive);
    startup_original_drive = (cb_u8)(current_drive - 1u);
    startup_original_directory[0] =
            (char)('A' + startup_original_drive);
    startup_original_directory[1] = ':';
    startup_original_directory[2] = '\\';

    registers.h.ah = 0x47u;
    registers.h.dl = (cb_u8)current_drive;
    destination = startup_original_directory + 3;
    segread(&segments);
    segments.ds = FP_SEG(destination);
    registers.x.si = FP_OFF(destination);
    (void)intdosx(&registers, &registers, &segments);

    source = startup_original_directory;
    destination = startup_original_file_path;
    do {
        character = *source++;
        *destination++ = character;
    } while (character != '\0');
    source_append = destination - 1;
    if (source_append[-1] != '\\') {
        *source_append++ = '\\';
    }

    source = startup_write_directory;
    destination = startup_write_file_path;
    do {
        character = *source++;
        *destination++ = character;
    } while (character != '\0');
    destination_append = destination - 1;
    if (destination_append[-1] != '\\') {
        *destination_append++ = '\\';
    }

    entry = resource_write_directory_names;
    do {
        startup_write_directory_enter();
        if (_dos_findfirst(entry->filename,
                STARTUP_COPY_FILE_ATTRIBUTES, &find_data) != 0u) {
            source = entry->filename;
            destination = source_append;
            do {
                character = *source++;
                *destination++ = character;
            } while (character != '\0');

            source = entry->filename;
            destination = destination_append;
            do {
                character = *source++;
                *destination++ = character;
            } while (character != '\0');

            startup_resource_file_copy(
                    startup_original_file_path,
                    startup_write_file_path);
        }
        ++entry;
    } while (entry->filename[0] != '\0');
}
