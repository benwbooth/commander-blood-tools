#include <conio.h>
#include <dos.h>
#include <stdlib.h>

#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_hardware.h"
#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

#define BLOODPRG_RESIDENT_PARAGRAPHS 0x14dau
#define BLOODPRG_MINIMUM_FREE_BYTES 0x00078870UL

void CB_FAR bloodprg_entry(void)
{
    union REGS registers;
    struct SREGS segments;
    cb_u16 largest_paragraphs;
    cb_u16 pool_segment;

    if (cpu_386_or_newer() == 0u) {
        print_string_dos(startup_cpu_error_text);
        exit(0);
    }

#if !defined(BLOODPRG_RELINKED_RUNTIME)
    segread(&segments);
    segments.es = _psp;
    registers.x.ax = 0x4a00u;
    registers.x.bx = BLOODPRG_RESIDENT_PARAGRAPHS;
    (void)intdosx(&registers, &registers, &segments);
#endif

    registers.x.ax = 0x4800u;
    registers.x.bx = 0xffffu;
    (void)intdos(&registers, &registers);
    largest_paragraphs = registers.x.bx;
    resource_free_bytes_gs = (cb_u32)largest_paragraphs << 4;
    if (resource_free_bytes_gs < BLOODPRG_MINIMUM_FREE_BYTES) {
        print_string_dos(startup_memory_error_text);
        goto release_pool;
    }

    registers.x.ax = 0x4800u;
    registers.x.bx = largest_paragraphs;
    (void)intdos(&registers, &registers);
    pool_segment = registers.x.ax;
    startup_dos_pool = (volatile cb_u8 CB_FAR *)MK_FP(pool_segment, 0u);
    resource_pool_end_segment = pool_segment;
    timer_state_block_offset = 0x0b29u;

    startup_command_line_parse(
            (const bloodprg_command_tail CB_FAR *)MK_FP(_psp, 0x0080u));
    mouse_reset_hide();
    cmos_rtc_read();
    install_ctrl_break_handler();
    install_timer_isr_hook();
    vga_mode_x_initialize();
    detect_cdrom();
    mouse_set_ranges(0u, 3000u, 0u, 200u);

    registers.x.ax = 4u;
    registers.x.cx = 720u;
    registers.x.dx = 150u;
    (void)int86(0x33, &registers, &registers);

    vga_retrace_phase_calibrate();
    extended_memory_backends_init();
    outp(0x43u, 0xb6u);
    outp(0x42u, 0x9cu);
    outp(0x42u, 0x2eu);
    bloodprg_main();
    extended_memory_backends_release();
    restore_timer_isr_hook();
    set_video_mode_saved();

release_pool:
    segread(&segments);
    segments.es = FP_SEG(startup_dos_pool);
    registers.x.ax = 0x4900u;
    (void)intdosx(&registers, &registers, &segments);
    exit(0);
}
