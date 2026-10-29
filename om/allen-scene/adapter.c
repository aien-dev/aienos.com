/* Temporary, explicitly authorized C-on-web boundary. Native implementation:
 * aien-dev/aienux@da55660ea7337f2b6a42095fcd1fb6ddebe1cc7c/native/kernel/ui.
 * Files in native/ are unchanged. Replace this adapter and those C units with
 * seeded Omega -> irwasm only after native-frame/audio parity, not by renaming C.
 * The web opening reuses native DUSK, then native dawn/formation; this ordering
 * is a web interaction, not the original two-hole boot sequence (replay=2).
 * No I/O, imports, heap allocator, threads, GPU interface or browser privileges.
 */
#include "native/night.h"
#include "native/mind.h"
#include "native/flow.h"
#include "native/chirp.h"
#include "native/sky_real.h"

void *memset(void *p, int c, size_t n) { unsigned char *q=p; while(n--) *q++=(unsigned char)c; return p; }
void *memcpy(void *d, const void *s, size_t n) { unsigned char *q=d; const unsigned char *p=s; while(n--) *q++=*p++; return d; }

static struct {
    ck_night night;
    ck_mind mind;
    ck_flow flow;
    ck_night_picture kept;
    ck_canvas cv;
    ck_af_face face;
    uint32_t *rgba, *back;
    void *scratch, *keep;
    int32_t w,h,cx,cy,far16;
    uint32_t k,t,kind,compact,word_t,key_t;
    int initialized;
} ui;
static int16_t audio[2*48000];
#define S(v) ((int32_t)(((int64_t)(v)*(int64_t)ui.k+128)/256))
#define EXPORT __attribute__((visibility("default")))

/* Verbatim state law from allen_ui.c:horizon_at; only struct ownership changes. */
static void horizon_at(uint32_t d)
{
    ck_night_dusk k = {0};
    uint32_t q = ck_smooth(ck_prog(d, 0, 3400));
    q = (q * q >> 8) * q >> 8;
    k.fallen = (int64_t)ui.far16 * ui.far16 * (int64_t)q >> 8;
    k.swirl = 65536u * 6 / 10;
    k.calm = S(120) * 16;
    k.crowd = 256 + (768 * ck_smooth(ck_prog(d, 300, 2400)) >> 8);
    k.e = (int32_t)((int64_t)S(90) * 16 * ck_smooth(ck_prog(d, 0, 2000)) >> 8);
    int32_t rs0 = (int32_t)((int64_t)S(60) * 16 * ck_smooth(ck_prog(d, 0, 2000)) >> 8);
    k.rs = rs0 + (int32_t)((int64_t)S(60) * 16 * ck_smooth(ck_prog(d, 3400, 4200)) >> 8);
    uint32_t rise = ck_smooth(ck_prog(d, 900, 2800)), fall = 256 - ck_smooth(ck_prog(d, 3100, 3900));
    k.disk = rise < fall ? rise : fall;
    k.beam = 16384u + d * 65536u / 9000u;
    k.keep = 256 - ck_ease_out(ck_prog(d, 2800, 4000));
    k.near = S(260) * 16, k.far = ui.far16;
    k.shade = 230 * ck_smooth(ck_prog(d, 400, 2600)) >> 8;
    k.fine = S(300) * 16;
    ck_night_fall(&ui.night, &k);
}

/* Verbatim merger flash from allen_ui.c:flash/unflash. */
static int flash(uint32_t t, int32_t *rad, uint32_t *a)
{
    if (t < CK_NIGHT_MERGE_MS || t >= CK_NIGHT_MERGE_MS + 1600) return 0;
    uint32_t tm = t - CK_NIGHT_MERGE_MS;
    *rad = (S(40) + (int32_t)((int64_t)S(220) * ck_ease_out(ck_prog(tm, 0, 1200)) >> 8)) * 16;
    uint32_t rise = ck_smooth(ck_prog(tm, 0, 120));
    *a = rise * (ck_exp_neg(tm > 120 ? (tm - 120) * 256 / 450 : 0) >> 8) * 230 >> 16;
    return *a > 0;
}
static void unflash(const ck_rect *r, uint32_t t)
{
    uint32_t tm=t-CK_NIGHT_MERGE_MS;
    if(ui.night.holes!=1 || tm<100) return;
    uint32_t keep=256-ck_smooth(ck_prog(tm,100,400));
    int32_t hx=ui.night.hole[0].x,hy=ui.night.hole[0].y,rs=ui.night.hole[0].rs;
    ck_rect box={(hx-rs)>>4,(hy-rs)>>4,(2*rs>>4)+2,(2*rs>>4)+2};
    if(rs<=16 || !ck_clip(&box,r)) return;
    for(int32_t y=box.y;y<box.y+box.h;y++) for(int32_t x=box.x;x<box.x+box.w;x++) {
        int32_t dx=x*16+8-hx,dy=y*16+8-hy;
        uint32_t d=ck_isqrt((uint32_t)(dx*dx+dy*dy));
        if(d>=(uint32_t)rs) continue;
        uint32_t in=(uint32_t)rs-d>=16?256:((uint32_t)rs-d)*16;
        uint32_t k=256-(in*(256-keep)>>8);
        ui.back[(size_t)y*ui.w+x]=ck_scale(ui.back[(size_t)y*ui.w+x],k);
    }
}

EXPORT int scene_init(uint32_t w,uint32_t h,uint32_t compact)
{
    if(w<64 || h<64 || w>1920 || h>1920 || (uint64_t)w*h>2073600) return 0;
    memset(&ui,0,sizeof ui);
    ui.w=w;ui.h=h;ui.compact=compact;
    ui.k=ck_min(w*256/1920,h*256/1080);
    /* Compact is a native design-scale crop, not a different particle scene. */
    if(compact) ui.k=ck_min(w,h)*256/400;
    ui.cx=w/2;ui.cy=compact?h/2:h*40/100;
    size_t pix=(size_t)w*h*4;
    size_t night=ck_night_mem_bytes(w,h,ui.k), kept=ck_night_picture_bytes(w,h);
    size_t needed=3*pix+night+kept+262144+64;
#ifdef __wasm__
    extern unsigned char __heap_base;
    uintptr_t base=((uintptr_t)&__heap_base+15)&~(uintptr_t)15;
    size_t end=base+needed, pages=(end+65535)/65536;
    size_t have=__builtin_wasm_memory_size(0);
    if(pages>have && __builtin_wasm_memory_grow(0,pages-have)==(size_t)-1) return 0;
    unsigned char *p=(unsigned char*)base;
#else
    static unsigned char arena[64*1024*1024];
    if(needed>sizeof arena) return 0;
    unsigned char *p=arena;
#endif
    ui.rgba=(uint32_t*)p;p+=pix;
    ui.back=(uint32_t*)p;p+=pix;
    ui.scratch=p;p+=pix;
    ui.cv=(ck_canvas){ui.back,(int32_t)w,(int32_t)h};
    if(ck_night_init(&ui.night,w,h,ui.k,ui.cx,ui.cy,p,night,ui.scratch)) return 0;
    p+=night;ui.keep=p;p+=kept;
    ck_af_face_init(&ui.face,ck_max(8,S(18)),1250,p,262144);
    for(int c=CK_AF_FIRST;c<=CK_AF_LAST;c++) (void)ck_af_glyph_get(&ui.face,(char)c);
    for(int i=0;i<4;i++) {
        int64_t dx=(i&1?ui.w:0)-ui.cx,dy=(i&2?ui.h:0)-ui.cy;
        ui.far16=ck_max(ui.far16,(int32_t)ck_isqrt((uint64_t)(dx*dx+dy*dy))*16);
    }
    ck_night_picture_take(&ui.night,ui.night.sky,ui.keep,&ui.kept);
    ck_mind_init(&ui.mind,ui.cx,ui.cy,ui.k,12300);
    ck_flow_init(&ui.flow,w,h,ui.cx,ui.cy,ui.mind.R,ui.cx,ui.h-S(177),0);
    ui.initialized=1;
    return 1;
}
EXPORT uintptr_t scene_pixels(void) { return (uintptr_t)ui.rgba; }
/* Native dusk consumes the whole preceding picture, not only the sky. For a
 * fresh web opening use the original unwoken UI at 20000 ms: no bar/words and
 * sleeping fluid, but the fully formed mind and twinkling sky are present.
 * oracle-ui.c independently exercises original allen_ui.c for this state. */
static void opening_picture(void)
{
    ck_rect all={0,0,ui.w,ui.h};
    ck_mind_init(&ui.mind,ui.cx,ui.cy,ui.k,12300);
    for(uint32_t t=0;t<=20000;t+=16) ck_mind_at(&ui.mind,t);
    ck_mind_at(&ui.mind,20000);
    ck_night_at(&ui.night,20000);
    ck_night_base(&ui.night,&ui.cv,&all);
    ck_night_lights(&ui.night,&ui.cv,&all,0,0);
    ck_mind_draw(&ui.mind,&ui.cv,&all);
    ck_night_picture_take(&ui.night,ui.back,ui.keep,&ui.kept);
}
EXPORT void scene_enter(uint32_t kind)
{
    if(!ui.initialized) return;
    ui.kind=kind>2?0:kind;ui.t=0;ui.word_t=ui.key_t=0;
    if(kind==1) opening_picture();
    ck_mind_init(&ui.mind,ui.cx,ui.cy,ui.k,kind==1?4200:12300);
    ck_flow_init(&ui.flow,ui.w,ui.h,ui.cx,ui.cy,ui.mind.R,ui.cx,ui.h-S(177),0);
    if(kind!=1) ck_night_picture_take(&ui.night,ui.night.sky,ui.keep,&ui.kept);
}
EXPORT void scene_key(uint32_t t) { if(ui.initialized) {ck_mind_key(&ui.mind,t);ck_flow_key(&ui.flow,t);ui.key_t=t;} }
EXPORT void scene_word(uint32_t t) { if(ui.initialized) {ck_mind_word(&ui.mind,t);ck_flow_word(&ui.flow,t);ui.word_t=t;} }
EXPORT void scene_step(uint32_t t,uint32_t mode,uint32_t voice)
{
    if(!ui.initialized) return;
    ui.t=t;
    ck_mind_mode(&ui.mind,(int)(mode&3),t);
    ck_mind_voice(&ui.mind,voice);
    int formed=t>=ui.mind.born+CK_MIND_FORM_MS;
    ck_flow_mode(&ui.flow,formed?(int)(mode&3):CK_FLOW_SLEEP,t);
    if(!ui.compact) ck_flow_at(&ui.flow,t);
    int32_t vx=0,vy=0;
    if(!ui.compact) ck_flow_velocity(&ui.flow,ui.cx,ui.cy,&vx,&vy);
    ui.mind.cx=ui.cx+ck_clamp(vx*S(10)/1000,-S(10),S(10));
    ui.mind.cy=ui.cy+ck_clamp(vy*S(10)/1000,-S(10),S(10));
    ck_mind_at(&ui.mind,t);
}
/* Native allen_ui.c per-word voice envelope; no browser floating approximation. */
EXPORT uint32_t scene_voice(uint32_t t,uint32_t word)
{
    return t>=word?ck_exp_neg((t-word)*256/200)>>8:0;
}
EXPORT void scene_draw(uint32_t shimmer,uint32_t voice)
{
    if(!ui.initialized) return;
    ck_rect all={0,0,ui.w,ui.h};
    uint32_t t=ui.t;
    ck_night_at(&ui.night,ui.kind==2?t:20000);
    if(ui.kind==1 && t<4200) {
        horizon_at(t);
        ck_night_base_from(&ui.night,&ui.kept,&ui.cv,&all);
    } else {
        ck_night_base(&ui.night,&ui.cv,&all);
        ck_night_lights(&ui.night,&ui.cv,&all,shimmer,voice*shimmer>>9);
        if(!ui.compact) ck_flow_draw(&ui.flow,&ui.cv,&all,256);
        int32_t rad;uint32_t a;
        if(ui.kind==2 && flash(t,&rad,&a)) {
            ck_glow(&ui.cv,&all,ui.cx*16+8,ui.cy*16+8,rad,CK_RGB(200,210,255),a/3);
            ck_glow(&ui.cv,&all,ui.cx*16+8,ui.cy*16+8,rad/3,CK_RGB(255,240,224),a);
            unflash(&all,t);
        }
        ck_mind_draw(&ui.mind,&ui.cv,&all);
        /* Native dawn duration is 900 ms; only its placement after dusk is web-specific. */
        if(ui.kind==1 && t<5100) {
            uint32_t gain=ck_prog(t,4200,5100);
            for(size_t i=0;i<(size_t)ui.w*ui.h;i++) ui.back[i]=ck_scale(ui.back[i],gain);
        }
    }
    for(size_t i=0;i<(size_t)ui.w*ui.h;i++) {
        uint32_t c=ui.back[i];
        ui.rgba[i]=0xff000000u|(c&0xff00u)|(c>>16&255u)|((c&255u)<<16);
    }
}
/* Pan/zoom the source's existing sky camera; no generated replacement stars. */
EXPORT int scene_view(int32_t ra,int32_t dec,int32_t fov)
{
    if(!ui.initialized || ra<0 || ra>360000000 || dec< -89000000 || dec>89000000 || fov<10000 || fov>120000) return 0;
    ck_sky_view v;
    if(ra==113697000 && dec== -72757000 && fov==80000) v=ck_sky_the_view;
    else ck_sky_view_at(&v,ra,dec,fov,0);
    ck_night_view(&ui.night,&v,ui.scratch);
    ck_night_picture_take(&ui.night,ui.night.sky,ui.keep,&ui.kept);
    return 1;
}
/* Native stroke glyph drawn above RGBA: no system or external font dependency.
 * Letters flying from the web input are an intentional browser enhancement. */
EXPORT void scene_glyph(uint32_t c,int32_t x,int32_t y,uint32_t alpha)
{
    if(!ui.initialized || c<32 || c>126 || alpha>256) return;
    ck_rect all={0,0,ui.w,ui.h};
    (void)ck_letter(&ui.cv,&all,&ui.face,(uint64_t)ck_max(0,x)*256,y,(char)c,CK_RGB(206,222,255),alpha,1);
    const ck_af_glyph *g=ck_af_glyph_get(&ui.face,(char)c);
    if(!g) return;
    ck_rect box={x+g->ox,y+g->oy,g->w,g->h};
    if(!ck_clip(&box,&all)) return;
    for(int32_t yy=box.y;yy<box.y+box.h;yy++) for(int32_t xx=box.x;xx<box.x+box.w;xx++) {
        size_t i=(size_t)yy*ui.w+xx;uint32_t col=ui.back[i];
        ui.rgba[i]=0xff000000u|(col&0xff00u)|(col>>16&255u)|((col&255u)<<16);
    }
}
EXPORT uintptr_t scene_audio(uint32_t from,uint32_t frames)
{
    if(frames>48000 || from>CK_CHIRP_FRAMES) return 0;
    ck_chirp_fill(audio,from,frames);return (uintptr_t)audio;
}
