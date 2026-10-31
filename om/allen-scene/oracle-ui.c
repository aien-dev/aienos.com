/* Independent host reference: include the ORIGINAL pinned allen_ui.c via -I.
 * Never link adapter.c here. Public tick/start/sleep APIs own frame history.
 * No wake/conversation is injected: the source's bar/panes remain hidden.
 */
#include <stdio.h>
#include <stdlib.h>
#include "allen_ui.c"
int main(int argc,char **argv)
{
    if(argc!=3) return 2;
    unsigned kind=(unsigned)strtoul(argv[1],0,10),target=(unsigned)strtoul(argv[2],0,10);
    const unsigned w=640,h=400;
    uint32_t *fb=calloc((size_t)w*h,4);
    size_t bytes=ck_allen_ui_mem_bytes(w,h);
    void *mem=calloc(1,bytes);
    if(!fb || !mem || ck_allen_ui_start(fb,w,h,w,0,mem,bytes,1000000,0,0)) return 3;
    unsigned end=kind==1?20000:target;
    for(unsigned t=0;t<=end;t+=16) ck_allen_ui_tick((uint64_t)t*1000);
    ck_allen_ui_refresh();ck_allen_ui_tick((uint64_t)end*1000);
    if(kind==1) {
        ck_allen_ui_sleep(1,20000000);
        for(unsigned d=0;d<=target;d+=16) ck_allen_ui_tick((uint64_t)(20016+d)*1000);
        ck_allen_ui_refresh();ck_allen_ui_tick((uint64_t)(20016+target)*1000);
    }
    /* Native framebuffer RGBX leaves X zero; browser RGBA requires opaque A. */
    for(size_t i=0;i<(size_t)w*h;i++) fb[i]|=0xff000000u;
    int result=fwrite(fb,4,(size_t)w*h,stdout)==(size_t)w*h?0:4;
    free(mem);free(fb);return result;
}
