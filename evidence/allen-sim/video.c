/* video.c -- records the kernel's own window, drawn by the kernel's own drawing code (native/kernel/ui,
 * snapshot 903277b), through a real conversation: the night, the merger, Allen forming, his first
 * welcome, three lines typed and answered by the kernel's own reply code (svc/allen_reply.c), then
 * goodbye (the dusk into the black hole) and the dawn. Writes raw RGBX frames (R byte first) of
 * 1280x720, every second 16 ms tick (31.25 a second), to the file given as argv[1], and the chirp as
 * 48 kHz stereo 16 bit little endian to argv[2]. build-video.sh turns them into the video. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "allen_reply.h"
#include "allen_ui.h"
#include "chirp.h"

#define W 1280u
#define H 720u
#define AT(x) (t >= (x) && t < (x) + 16)

static ck_allen_mind mind;

struct line {
    uint32_t at;
    const char *text;
};
static const struct line LINES[] = {
    {21008, "Allen"},
    {29008, "what can you do"},
    {39008, "what is two plus three"},
    {47008, "goodbye"},
};

int main(int argc, char **argv)
{
    if (argc < 3)
        return 2;
    size_t need = ck_allen_ui_mem_bytes(W, H);
    void *mem = calloc(1, need);
    uint32_t *fb = calloc((size_t)W * H, 4);
    if (ck_allen_ui_start(fb, W, H, W, 0, mem, need, 1000, 0, 0))
        return 1;
    ck_allen_facts facts = {4, 0, 1, W, H};
    ck_allen_mind_init(&mind, &facts);
    FILE *out = fopen(argv[1], "wb");
    char typed[160];
    const uint32_t end = 70000;
    uint32_t sleep_at = 0, wake_at = 0;
    for (uint32_t t = 0; t <= end; t += 16) {
        if (AT(14000))
            ck_allen_ui_wake("Say anything", t);
        if (AT(15920))
            ck_allen_ui_said(0, ck_allen_welcome(&mind), t);
        for (unsigned i = 0; i < sizeof LINES / sizeof *LINES; i++) {
            uint32_t at = LINES[i].at, n = (uint32_t)strlen(LINES[i].text);
            if (t >= at && t < at + n * 140 && (t - at) % 140 < 16) {
                unsigned k = (t - at) / 140 + 1;
                memcpy(typed, LINES[i].text, k);
                typed[k] = 0;
                ck_allen_ui_input(typed, k, t);
            }
            if (AT(at + n * 140 + 400)) {
                const char *reply = ck_allen_answer(&mind, LINES[i].text, t / 1000);
                ck_allen_ui_input("", 0, t);
                ck_allen_ui_said(LINES[i].text, reply, t);
                if (reply == ck_allen_goodbye)
                    sleep_at = t + 3400, wake_at = t + 3400 + 4200 + 3500;
            }
        }
        if (sleep_at && AT(sleep_at))
            ck_allen_ui_sleep(1, t);
        if (wake_at && AT(wake_at))
            ck_allen_ui_sleep(0, t);
        ck_allen_ui_tick(t);
        if ((t / 16) % 2 == 0)
            fwrite(fb, 4, (size_t)W * H, out);
    }
    fclose(out);
    FILE *a = fopen(argv[2], "wb");
    static int16_t buf[2 * 4096];
    for (uint32_t f = 0; f < (uint32_t)(end * 48); f += 4096) {
        uint32_t n = (uint32_t)(end * 48) - f < 4096 ? (uint32_t)(end * 48) - f : 4096;
        ck_chirp_fill(buf, f, n);
        fwrite(buf, 4, n, a);
    }
    fclose(a);
    return 0;
}
