/* reference.c -- drives the kernel's own Allen window code (native/kernel/ui at 903277b) through a
 * fixed script on a hand-moved clock and prints an FNV-1a hash of the screen after every 4th frame
 * (alpha byte masked off); one size per process (the kernel keeps some state between windows, which a fresh page does not). The Rust port (crates/allen-sim, tests/reference.rs) runs the same script
 * and must print the same lines. Also prints the chirp sound's hash. Build: see build-reference.sh. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "allen_ui.h"
#include "chirp.h"

static uint64_t hash_frame(const uint32_t *p, size_t n)
{
    uint64_t h = 1469598103934665603ull;
    for (size_t i = 0; i < n; i++) {
        uint32_t v = p[i] & 0x00ffffffu;
        for (int b = 0; b < 3; b++) {
            h ^= (v >> (8 * b)) & 255;
            h *= 1099511628211ull;
        }
    }
    return h;
}

#define AT(x) (t >= (x) && t < (x) + 16)
static const uint8_t EA[] = {0, 0, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 2, 5};
static const uint8_t EB[] = {1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 7, 9};

static uint32_t dump_t;
static const char *dump_path;
static void run(uint32_t w, uint32_t h, int report)
{
    size_t need = ck_allen_ui_mem_bytes(w, h);
    void *mem = calloc(1, need);
    uint32_t *fb = calloc((size_t)w * h, 4);
    if (ck_allen_ui_start(fb, w, h, w, 0, mem, need, 1000, 0, 0)) {
        printf("start failed\n");
        exit(1);
    }
    if (report) printf("size %ux%u\n", w, h);
    const char *typed = "Hello Allen";
    for (uint32_t t = 0; t <= 44000; t += 16) {
        if (AT(14000)) ck_allen_ui_wake("Say anything", t);
        if (t >= 15000 && t < 15000 + 11 * 140 && (t - 15000) % 140 < 16) {
            unsigned n = (t - 15000) / 140 + 1;
            ck_allen_ui_input(typed, n, t);
        }
        if (AT(17000)) ck_allen_ui_said("Hello Allen", "Hello Humanity. I am here, and the night is clear.", t);
        if (AT(25000)) ck_allen_ui_map(12, EA, EB, 15, 3, "a note about the night sky", 12, 15);
        if (AT(30000)) ck_allen_ui_input("what is", 7, t);
        if (AT(31000)) ck_allen_ui_think("what is water", t);
        if (AT(33000)) ck_allen_ui_said(0, "Water is where I live.", t);
        if (AT(36000)) ck_allen_ui_sleep(1, t);
        if (AT(41000)) ck_allen_ui_sleep(0, t);
        int drew = ck_allen_ui_tick(t);
        if (!report && t == dump_t) {
            FILE *f = fopen(dump_path, "wb");
            fwrite(fb, 4, (size_t)w * h, f);
            fclose(f);
        }
        if (report && (t / 16) % 4 == 0)
            printf("%u %d %016llx\n", t, drew, (unsigned long long)hash_frame(fb, (size_t)w * h));
    }
    if (report) printf("asleep %d on %d\n", ck_allen_ui_asleep(), ck_allen_ui_on());
    free(mem);
    free(fb);
}

int main(int argc, char **argv)
{
    if (argc > 3) {
        dump_t = (uint32_t)atoi(argv[2]);
        dump_path = argv[3];
        run((uint32_t)atoi(argv[1]), (uint32_t)atoi(argv[1]) == 800 ? 500 : (uint32_t)atoi(argv[1]) * 9 / 16, 0);
        return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "1280")) {
        run(1280, 720, 1);
        return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "800")) {
        run(800, 500, 1);
        return 0;
    }
    uint64_t h = 1469598103934665603ull;
    static int16_t buf[2 * 4096];
    for (uint32_t f = 0; f < CK_CHIRP_FRAMES; f += 4096) {
        uint32_t n = CK_CHIRP_FRAMES - f < 4096 ? CK_CHIRP_FRAMES - f : 4096;
        ck_chirp_fill(buf, f, n);
        for (uint32_t i = 0; i < 2 * n; i++) {
            h ^= (uint16_t)buf[i] & 255; h *= 1099511628211ull;
            h ^= (uint16_t)buf[i] >> 8; h *= 1099511628211ull;
        }
    }
    printf("chirp %016llx\n", (unsigned long long)h);
    return 0;
}
