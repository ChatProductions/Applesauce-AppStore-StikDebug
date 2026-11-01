#include <psp2/display.h>
#include <psp2/ctrl.h>
#include <psp2/kernel/threadmgr.h>
#include <psp2/kernel/sysmem.h>
#include <stdint.h>
#include <stdlib.h>
#include <malloc.h>
#include "quickjs.h"
#include <stdio.h>
#include <string.h>
static int jsok=0;
static int tested=0;
static int input=7;
static int js_result=0;
static int c_result=0;

static int test_num(JSContext *ctx,const char *src,int expected,int *observed){
  JSValue v=JS_Eval(ctx,src,strlen(src),"probe",JS_EVAL_TYPE_GLOBAL);
  int32_t x=0;
  int valid=!JS_IsException(v) && JS_ToInt32(ctx,&x,v)==0 && x==expected;
  if(observed)*observed=x;
  JS_FreeValue(ctx,v);
  return valid;
}
static void run_tests(void){
  tested=0; jsok=0; js_result=-1; c_result=30+input;
  JSRuntime *rt=JS_NewRuntime();
  if(!rt)return;
  JSContext *ctx=JS_NewContext(rt);
  if(ctx){
    char code[256];
    snprintf(code,sizeof(code),"(function(){let a=[1,2,3,4];return a.map(x=>x*x).reduce((a,b)=>a+b,0)+%d})()",input);
    tested+=test_num(ctx,code,c_result,&js_result);
    snprintf(code,sizeof(code),"(function(){const sq=x=>x*x;return sq(%d)})()",input);
    tested+=test_num(ctx,code,input*input,NULL);
    tested+=test_num(ctx,"JSON.parse('[2,3,5]').reduce((a,b)=>a+b,0)",10,NULL);
    tested+=test_num(ctx,"(/^[a-z]+$/.test('vitasurf')?1:0)",1,NULL);
    tested+=test_num(ctx,"(()=>{try{throw 7}catch(e){return e}})()",7,NULL);
    tested+=test_num(ctx,"(()=>{let n=0;for(let i=0;i<2000;i++)n+=(i&1);return n})()",1000,NULL);
    JS_FreeContext(ctx);
  }
  JS_FreeRuntime(rt);
  jsok=(tested==6);
}

#define W 960
#define H 544
static uint32_t *fb;
static const uint8_t font[18][7] = {
{14,17,17,31,17,17,17}, /* A */
{30,17,17,30,17,17,30}, /* B */
{14,17,16,16,16,17,14}, /* C */
{30,17,17,17,17,17,30}, /* D */
{31,16,16,30,16,16,31}, /* E */
{31,16,16,30,16,16,16}, /* F */
{14,17,16,23,17,17,14}, /* G */
{17,17,17,31,17,17,17}, /* H */
{31,4,4,4,4,4,31}, /* I */
{7,2,2,2,18,18,12}, /* J */
{17,18,20,24,20,18,17}, /* K */
{16,16,16,16,16,16,31}, /* L */
{17,27,21,21,17,17,17}, /* M */
{17,25,21,19,17,17,17}, /* N */
{14,17,17,17,17,17,14}, /* O */
{30,17,17,30,16,16,16}, /* P */
{14,17,17,17,21,18,13}, /* Q */
{30,17,17,30,20,18,17} /* R */
};
static uint8_t glyph(char c,int y) {
  if(c>='A'&&c<='R') return font[c-'A'][y];
  if(c=='S') { static const uint8_t a[]={15,16,16,14,1,1,30};return a[y]; }
  if(c=='T') return y==0?31:4;
  if(c=='U') return y==6?14:17;
  if(c=='V') return y<5?17:(y==5?10:4);
  if(c=='W') return y<4?17:(y==4?21:(y==5?27:17));
  if(c=='X') return y==3?4:(y==2||y==4?10:17);
  if(c=='Y') return y<3?17:(y==3?10:4);
  if(c=='Z') return y==0||y==6?31:(1<<(6-y));
  if(c>='0'&&c<='9') { static const uint8_t nums[10][7]={{14,17,19,21,25,17,14},{4,12,4,4,4,4,14},{14,17,1,2,4,8,31},{30,1,1,14,1,1,30},{2,6,10,18,31,2,2},{31,16,16,30,1,1,30},{14,16,16,30,17,17,14},{31,1,2,4,8,8,8},{14,17,17,14,17,17,14},{14,17,17,15,1,1,14}};return nums[c-'0'][y];}
  if(c==':') return y==2||y==5?4:0;
  if(c=='.') return y==6?4:0;
  if(c=='-') return y==3?31:0;
  return 0;
}
static void rect(int x,int y,int w,int h,uint32_t color) {
  for(int j=y;j<y+h&&j<H;j++) for(int i=x;i<x+w&&i<W;i++) if(i>=0&&j>=0) fb[j*W+i]=color;
}
static void label(int x,int y,const char *s,uint32_t color,int scale) {
  while(*s){ for(int j=0;j<7;j++){uint8_t bits=glyph(*s,j);for(int i=0;i<5;i++)if(bits&(1<<(4-i)))rect(x+i*scale,y+j*scale,scale,scale,color);} x+=6*scale;s++; }
}
static void draw(int mode,unsigned frames) {
  uint32_t bg=mode?0xff132c30:0xff191527;
  rect(0,0,W,H,bg);
  rect(0,0,W,12,0xff34d5bd);
  label(60,60,"VITASURF",0xffffffff,5);
  label(60,125,"QUICKJS ENGINE TEST",0xff34d5bd,3);
  label(60,205,"CROSS: CHANGE INPUT",0xffffffff,3);
  label(60,248,"START: EXIT",0xffffffff,3);
  label(60,310,jsok?"JS PASS":"JS FAIL",jsok?0xff34d5bd:0xffff6666,4);
  char msg[100];
  snprintf(msg,sizeof(msg),"TESTS %d OF 6",tested);
  label(60,364,msg,0xffffffff,2);
  snprintf(msg,sizeof(msg),"INPUT %d JS %d C %d",input,js_result,c_result);
  label(60,396,msg,0xffffffff,2);
  rect(60,453,840,30,mode?0xfff1b04c:0xff34d5bd);
  (void)frames;
}
int main(void) {
  run_tests();
  const unsigned bytes=W*H*sizeof(uint32_t);
  const unsigned stride=(bytes+1024*1024-1)&~(1024*1024-1);
  const unsigned alloc_bytes=2*stride;
  SceUID memblock=sceKernelAllocMemBlock("VitaSurfFrame",SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW,alloc_bytes,NULL);
  if(memblock<0)return -1;
  void *base=NULL;
  if(sceKernelGetMemBlockBase(memblock,&base)<0){sceKernelFreeMemBlock(memblock);return -2;}
  uint32_t *screens[2]={(uint32_t *)base,(uint32_t *)((char *)base+stride)};
  for(int i=0;i<2;i++) { fb=screens[i]; draw(i,0); }
  SceDisplayFrameBuf frame={0};
  frame.size=sizeof(frame);
  frame.base=fb;
  frame.pitch=W;
  frame.pixelformat=SCE_DISPLAY_PIXELFORMAT_A8B8G8R8;
  frame.width=W;
  frame.height=H;
  sceCtrlSetSamplingMode(SCE_CTRL_MODE_DIGITAL);
  int mode=0;unsigned prev=0;
  for(unsigned frames=0;;frames++){
    SceCtrlData pad={0};
    sceCtrlPeekBufferPositive(0,&pad,1);
    if(pad.buttons&SCE_CTRL_START)break;
    if((pad.buttons&SCE_CTRL_CROSS)&&!(prev&SCE_CTRL_CROSS)){
      input=(input*17+23)%997+1;
      run_tests();
      mode=!mode;
      fb=screens[mode];
      draw(mode,frames);
    }
    prev=pad.buttons;
    frame.base=screens[mode];
    sceDisplaySetFrameBuf(&frame,SCE_DISPLAY_SETBUF_NEXTFRAME);
    sceDisplayWaitVblankStart();
  }
  sceKernelFreeMemBlock(memblock);
  return 0;
}
