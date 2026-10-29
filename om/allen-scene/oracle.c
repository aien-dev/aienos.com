/* Host-only read-only oracle driver; never shipped as a browser dependency.
 * Link adapter.c against the exact original native pin, not a replacement renderer.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int scene_init(uint32_t,uint32_t,uint32_t);
uintptr_t scene_pixels(void);
void scene_enter(uint32_t);
void scene_step(uint32_t,uint32_t,uint32_t);
void scene_draw(uint32_t,uint32_t);
void scene_key(uint32_t);
void scene_word(uint32_t);
void scene_glyph(uint32_t,int32_t,int32_t,uint32_t);
int scene_view(int32_t,int32_t,int32_t);
uintptr_t scene_audio(uint32_t,uint32_t);
int main(int argc,char **argv)
{
    if(argc==2 && !strcmp(argv[1],"audio")) {
        uintptr_t p=scene_audio(11360*48,45120);
        return p && fwrite((void*)p,4,45120,stdout)==45120?0:2;
    }
    if(argc!=5) return 3;
    uint32_t kind=(uint32_t)strtoul(argv[1],0,10),target=(uint32_t)strtoul(argv[2],0,10);
    uint32_t compact=(uint32_t)strtoul(argv[3],0,10),variant=(uint32_t)strtoul(argv[4],0,10);
    if(!scene_init(640,400,compact)) return 4;
    scene_enter(kind);
    if(variant==2 && !scene_view(119000000,-71000000,60000)) return 5;
    for(uint32_t t=0;t<=target;t+=16) {
        if(variant==1 && t==16000) scene_key(t);
        if(variant==1 && t==17008) scene_word(t);
        scene_step(t,variant==1?(t<16000?0:t<17008?1:3):0,variant==1?128:0);
    }
    scene_step(target,variant==1?3:0,variant==1?128:0);
    scene_draw(variant==1?256:0,variant==1?128:0);
    if(variant==1) scene_glyph('A',300,300,200);
    return fwrite((void*)scene_pixels(),4,640*400,stdout)==640*400?0:6;
}
