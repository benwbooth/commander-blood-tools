/* Codegen probe for BLOODPRG 0x000600. */

#include <conio.h>
#include <dos.h>
#include <stdlib.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#define FAR far
#define NEAR near

typedef struct command_tail_probe {
    u8 length;
    char text[127];
} command_tail_probe;

extern const char cpu_error_text_probe[];
extern const char memory_error_text_probe[];
extern volatile u8 FAR *dos_pool_probe;
extern volatile u32 resource_free_bytes_probe;
extern volatile u16 resource_pool_end_segment_probe;
extern volatile u16 reserved_word_0af0_probe;

extern u16 FAR cpu_386_or_newer_probe(void);
extern void FAR print_string_dos_probe(const volatile char *text);
extern void NEAR startup_command_line_parse_probe(
        const command_tail_probe FAR *command_tail);
extern void FAR mouse_reset_hide_probe(void);
extern void FAR cmos_rtc_read_probe(void);
extern void FAR install_ctrl_break_handler_probe(void);
extern void FAR install_timer_isr_hook_probe(void);
extern void FAR vga_mode_x_initialize_probe(void);
extern void NEAR detect_cdrom_probe(void);
extern void FAR mouse_set_ranges_probe(
        u16 min_x, u16 max_x, u16 min_y, u16 max_y);
extern void FAR vga_retrace_phase_calibrate_probe(void);
extern void FAR extended_memory_backends_init_probe(void);
extern void FAR bloodprg_main_probe(void);
extern void FAR extended_memory_backends_release_probe(void);
extern void FAR restore_timer_isr_hook_probe(void);
extern void FAR set_video_mode_saved_probe(void);

void FAR bloodprg_entry_probe(void)
{
    union REGS registers;
    struct SREGS segments;
    u16 largest_paragraphs;
    u16 pool_segment;

    if (cpu_386_or_newer_probe() == 0u) {
        print_string_dos_probe(cpu_error_text_probe);
        exit(0);
    }

    segread(&segments);
    segments.es = _psp;
    registers.x.ax = 0x4a00u;
    registers.x.bx = 0x14dau;
    (void)intdosx(&registers, &registers, &segments);

    registers.x.ax = 0x4800u;
    registers.x.bx = 0xffffu;
    (void)intdos(&registers, &registers);
    largest_paragraphs = registers.x.bx;
    resource_free_bytes_probe = (u32)largest_paragraphs << 4;
    if (resource_free_bytes_probe < 0x00078870UL) {
        print_string_dos_probe(memory_error_text_probe);
        goto release_pool;
    }

    registers.x.ax = 0x4800u;
    registers.x.bx = largest_paragraphs;
    (void)intdos(&registers, &registers);
    pool_segment = registers.x.ax;
    dos_pool_probe = (volatile u8 FAR *)MK_FP(pool_segment, 0u);
    resource_pool_end_segment_probe = pool_segment;
    reserved_word_0af0_probe = 0x0b29u;

    startup_command_line_parse_probe(
            (const command_tail_probe FAR *)MK_FP(_psp, 0x0080u));
    mouse_reset_hide_probe();
    cmos_rtc_read_probe();
    install_ctrl_break_handler_probe();
    install_timer_isr_hook_probe();
    vga_mode_x_initialize_probe();
    detect_cdrom_probe();
    mouse_set_ranges_probe(0u, 3000u, 0u, 200u);

    registers.x.ax = 4u;
    registers.x.cx = 720u;
    registers.x.dx = 150u;
    (void)int86(0x33, &registers, &registers);

    vga_retrace_phase_calibrate_probe();
    extended_memory_backends_init_probe();
    outp(0x43u, 0xb6u);
    outp(0x42u, 0x9cu);
    outp(0x42u, 0x2eu);
    bloodprg_main_probe();
    extended_memory_backends_release_probe();
    restore_timer_isr_hook_probe();
    set_video_mode_saved_probe();

release_pool:
    segread(&segments);
    segments.es = FP_SEG(dos_pool_probe);
    registers.x.ax = 0x4900u;
    (void)intdosx(&registers, &registers, &segments);
    exit(0);
}
