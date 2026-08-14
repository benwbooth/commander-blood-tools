/* Codegen probe for BLOODPRG 0x0016A7. */

#include <direct.h>
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

typedef volatile u8 FAR *graphics_buffer_ptr;

typedef struct resource_name_entry_probe {
    char filename[16];
} resource_name_entry_probe;

extern volatile u8 bridge_panorama_palette_probe[768];
extern graphics_buffer_ptr graphics_draw_framebuffer_probe;
extern graphics_buffer_ptr graphics_screen_buffer_probe;
extern graphics_buffer_ptr graphics_display_buffer_probe;
extern const u8 startup_loading_text_probe[];
extern char startup_write_directory_probe[32];
extern char startup_original_directory_probe[32];
extern char startup_original_file_path_probe[32];
extern char startup_write_file_path_probe[32];
extern u8 startup_write_drive_probe;
extern u8 startup_original_drive_probe;
extern const resource_name_entry_probe resource_names_probe[];

extern void FAR vga_palette_write_probe(const volatile u8 *palette);
extern void FAR blit_fill_row_probe(u8 color);
extern const u8 FAR *FAR font8x8_text_draw_probe(
        const u8 FAR *text, u16 x, u16 y, u16 color_and_limit);
extern void FAR chunky_to_planar_probe(
        const volatile u8 FAR *source);
extern void FAR write_directory_enter_probe(void);
extern void FAR resource_file_copy_probe(
        volatile char FAR *source_path,
        const volatile char FAR *destination_path);

void NEAR startup_loading_screen_and_write_directory_prepare_probe(void)
{
    graphics_buffer_ptr saved_draw_framebuffer;
    const resource_name_entry_probe *entry;
    struct find_t find_data;
    union REGS registers;
    struct SREGS segments;
    char *source_append;
    char *destination_append;
    const char *source;
    char *destination;
    char character;
    unsigned current_drive;

    vga_palette_write_probe(bridge_panorama_palette_probe);
    blit_fill_row_probe(0u);
    (void)font8x8_text_draw_probe(
            startup_loading_text_probe, 130u, 96u, 0xffefu);

    saved_draw_framebuffer = graphics_draw_framebuffer_probe;
    graphics_draw_framebuffer_probe = graphics_screen_buffer_probe;
    chunky_to_planar_probe(graphics_display_buffer_probe);
    graphics_draw_framebuffer_probe = saved_draw_framebuffer;

    (void)mkdir(startup_write_directory_probe);
    startup_write_drive_probe =
            (u8)(startup_write_directory_probe[0] - 'A');

    _dos_getdrive(&current_drive);
    startup_original_drive_probe = (u8)(current_drive - 1u);
    startup_original_directory_probe[0] =
            (char)('A' + startup_original_drive_probe);
    startup_original_directory_probe[1] = ':';
    startup_original_directory_probe[2] = '\\';

    registers.h.ah = 0x47u;
    registers.h.dl = (u8)current_drive;
    destination = startup_original_directory_probe + 3;
    segread(&segments);
    segments.ds = FP_SEG(destination);
    registers.x.si = FP_OFF(destination);
    (void)intdosx(&registers, &registers, &segments);

    source = startup_original_directory_probe;
    destination = startup_original_file_path_probe;
    do {
        character = *source++;
        *destination++ = character;
    } while (character != '\0');
    source_append = destination - 1;
    if (source_append[-1] != '\\') {
        *source_append++ = '\\';
    }

    source = startup_write_directory_probe;
    destination = startup_write_file_path_probe;
    do {
        character = *source++;
        *destination++ = character;
    } while (character != '\0');
    destination_append = destination - 1;
    if (destination_append[-1] != '\\') {
        *destination_append++ = '\\';
    }

    entry = resource_names_probe;
    do {
        write_directory_enter_probe();
        if (_dos_findfirst(entry->filename, 0x18u, &find_data) != 0u) {
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

            resource_file_copy_probe(
                    startup_original_file_path_probe,
                    startup_write_file_path_probe);
        }
        ++entry;
    } while (entry->filename[0] != '\0');
}
