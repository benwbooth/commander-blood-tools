#include <dos.h>
#include <stdio.h>

static int dos_call_far_path(
        union REGS *registers,
        struct SREGS *segments,
        const volatile char __far *path)
{
    segread(segments);
    segments->ds = FP_SEG(path);
    registers->x.dx = FP_OFF(path);
    int86x(0x21, registers, registers, segments);
    return registers->x.cflag == 0;
}

static int dos_open_read_only(
        const volatile char __far *path, unsigned *handle)
{
    union REGS registers;
    struct SREGS segments;

    registers.x.ax = 0x3d00u;
    if (dos_call_far_path(&registers, &segments, path)) {
        *handle = registers.x.ax;
        return 1;
    }
    *handle = registers.x.ax;
    return 0;
}

int main(void)
{
    static const char missing_path[] = "MISSING.NO";
    const volatile char __far *path = missing_path;
    FILE *result;
    unsigned handle = 0x24c3u;
    int opened;

    opened = dos_open_read_only(path, &handle);
    result = fopen("RESULT.TXT", "wt");
    if (result == NULL) {
        return 2;
    }
    if (!opened && (handle == 2u || handle == 3u)) {
        fputs("PASS bloodprg DOS open\n", result);
    } else {
        fprintf(result, "FAIL opened=%d handle=%04X\n", opened, handle);
    }
    fclose(result);
    return 0;
}
