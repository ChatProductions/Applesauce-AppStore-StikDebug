#include <psp2/display.h>
#include <psp2/ctrl.h>
#include <psp2/kernel/threadmgr.h>
#include <psp2/kernel/sysmem.h>
#include <stdint.h>
#include <stdlib.h>
#include <malloc.h>
#include <string.h>
#include <psp2/sysmodule.h>
#include <psp2/net/net.h>
#include <psp2/net/netctl.h>
#include <psp2/net/http.h>
#include <psp2/libssl.h>
#include <stdio.h>
#include <ctype.h>

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

#define PAGE_LIMIT (128 * 1024)
static int stage=0;
static int failure=0;
static int http_code=0;
static int received=0;
static int html_seen=0;
static int has_title=0;
static int requests=0;
static int phase=0;
static char title_text[53]="NO TITLE";

/* Keep the Vita networking and TLS stacks alive across repeated requests. */
static char *net_mem=NULL;
static int net_module=0, https_module=0;
static int net_ready=0, ctl_ready=0, http_ready=0, ssl_ready=0;
static int initialized=0;

static void network_shutdown(void) {
  if(ssl_ready){sceSslTerm();ssl_ready=0;}
  if(http_ready){sceHttpTerm();http_ready=0;}
  if(ctl_ready){sceNetCtlTerm();ctl_ready=0;}
  if(net_ready){sceNetTerm();net_ready=0;}
  if(https_module){sceSysmoduleUnloadModule(SCE_SYSMODULE_HTTPS);https_module=0;}
  if(net_module){sceSysmoduleUnloadModule(SCE_SYSMODULE_NET);net_module=0;}
  free(net_mem);
  net_mem=NULL;
  initialized=0;
}

static int network_startup(void) {
  if(initialized)return 0;
  int r;
  phase=1;
  r=sceSysmoduleLoadModule(SCE_SYSMODULE_NET);
  if(r<0)goto fail;
  net_module=1;
  phase=2;
  r=sceSysmoduleLoadModule(SCE_SYSMODULE_HTTPS);
  if(r<0)goto fail;
  https_module=1;
  phase=3;
  net_mem=(char *)malloc(1024*1024);
  if(!net_mem){r=-2;goto fail;}
  SceNetInitParam net={0};
  net.memory=net_mem;net.size=1024*1024;net.flags=0;
  r=sceNetInit(&net);
  if(r<0)goto fail;
  net_ready=1;
  phase=4;
  r=sceNetCtlInit();
  if(r<0)goto fail;
  ctl_ready=1;
  phase=5;
  r=sceHttpInit(1024*1024);
  if(r<0)goto fail;
  http_ready=1;
  phase=6;
  r=sceSslInit(4*1024*1024);
  if(r<0)goto fail;
  ssl_ready=1;
  initialized=1;
  return 0;
fail:
  network_shutdown();
  return r;
}

static int ci_equal(char a,char b) {
  return toupper((unsigned char)a)==toupper((unsigned char)b);
}
static int find_ci(const char *data,int len,const char *needle) {
  int n=(int)strlen(needle);
  for(int i=0;i+n<=len;i++) {
    int ok=1;
    for(int j=0;j<n;j++) if(!ci_equal(data[i+j],needle[j])){ok=0;break;}
    if(ok)return i;
  }
  return -1;
}
static void parse_response(const char *data,int len) {
  html_seen=find_ci(data,len,"<html")>=0 || find_ci(data,len,"<!doctype html")>=0;
  has_title=0;
  strcpy(title_text,"NO TITLE");
  int pos=find_ci(data,len,"<title");
  if(pos<0)return;
  int start=pos+6;
  while(start<len && data[start]!='>')start++;
  if(start>=len)return;
  start++;
  int end=find_ci(data+start,len-start,"</title>");
  if(end<0)return;
  int limit=(int)sizeof(title_text)-1;
  int out=0;
  for(int i=start;i<start+end && out<limit;i++) {
    char c=data[i];
    if(c=='\r'||c=='\n'||c=='\t')c=' ';
    if(c>='a'&&c<='z')c=(char)(c-'a'+'A');
    if((c>='A'&&c<='Z')||(c>='0'&&c<='9')||c==' '||c=='.'||c=='-')
      title_text[out++]=c;
  }
  title_text[out]=0;
  has_title=out>0;
}
static void do_fetch(void) {
  requests++;
  stage=1;failure=0;http_code=0;received=0;html_seen=0;has_title=0;
  strcpy(title_text,"NO TITLE");
  int tpl=-1,conn=-1,req=-1;
  char *page=NULL;
  int r=network_startup();
  if(r<0){failure=r;stage=3;return;}
  phase=7;
  page=(char *)malloc(PAGE_LIMIT+1);
  if(!page){failure=-2;goto cleanup;}
  tpl=sceHttpCreateTemplate("VitaSurf/0.3",SCE_HTTP_VERSION_1_1,SCE_HTTP_PROXY_AUTO);
  if(tpl<0){failure=tpl;goto cleanup;}
  sceHttpSetResolveTimeOut(tpl,8000000);
  sceHttpSetConnectTimeOut(tpl,8000000);
  sceHttpSetRecvTimeOut(tpl,8000000);
  sceHttpSetSendTimeOut(tpl,8000000);
  sceHttpSetAutoRedirect(tpl,1);
  sceHttpAddRequestHeader(tpl,"Accept","text/html",SCE_HTTP_HEADER_ADD);
  const char *url="https://example.com/";
  phase=8;
  conn=sceHttpCreateConnectionWithURL(tpl,url,0);
  if(conn<0){failure=conn;goto cleanup;}
  phase=9;
  req=sceHttpCreateRequestWithURL(conn,SCE_HTTP_METHOD_GET,url,0);
  if(req<0){failure=req;goto cleanup;}
  phase=10;
  r=sceHttpSendRequest(req,NULL,0);
  if(r<0){failure=r;goto cleanup;}
  phase=11;
  r=sceHttpGetStatusCode(req,&http_code);
  if(r<0){failure=r;goto cleanup;}
  phase=12;
  while(received<PAGE_LIMIT){
    int room=PAGE_LIMIT-received;
    int step=room>4096?4096:room;
    r=sceHttpReadData(req,page+received,(unsigned)step);
    if(r<0){failure=r;break;}
    if(r==0)break;
    received+=r;
  }
  page[received]=0;
  if(received>0)parse_response(page,received);
cleanup:
  if(req>=0){if(failure)sceHttpAbortRequest(req);sceHttpDeleteRequest(req);}
  if(conn>=0)sceHttpDeleteConnection(conn);
  if(tpl>=0)sceHttpDeleteTemplate(tpl);
  free(page);
  stage=(failure==0 && http_code==200 && html_seen && has_title)?2:3;
}
static void draw(int mode,unsigned frames){
  rect(0,0,W,H,mode?0xff132c30:0xff191527);
  rect(0,0,W,12,0xff34d5bd);
  label(60,54,"VITASURF",0xffffffff,5);
  label(60,120,"HTTPS FETCH TEST",0xff34d5bd,3);
  label(60,175,"CROSS: FETCH PAGE",0xffffffff,3);
  label(60,218,"START: EXIT",0xffffffff,3);
  label(60,280,stage==0?"READY":stage==1?"CONNECTING":stage==2?"HTTPS PASS":"HTTPS FAIL",
    stage==2?0xff34d5bd:(stage==3?0xffff6677:0xffffffff),4);
  char line[80];
  snprintf(line,sizeof(line),"HTTP %d  BYTES %d",http_code,received);
  label(60,345,line,0xffffffff,2);
  snprintf(line,sizeof(line),"HTML %d TITLE %d",html_seen,has_title);
  label(60,377,line,0xffffffff,2);
  if(failure){
    snprintf(line,sizeof(line),"ERROR %08X",(unsigned)failure);
    label(60,414,line,0xffff6677,2);
  }else if(stage==2){
    label(60,414,title_text,0xff34d5bd,2);
  }
  snprintf(line,sizeof(line),"REQUEST %d PHASE %d",requests,phase);
  label(60,451,line,0xffffffff,2);
  rect(60,490,840,17,mode?0xfff1b04c:0xff34d5bd);
  (void)frames;
}
int main(void){
  const unsigned bytes=W*H*sizeof(uint32_t);
  const unsigned stride=(bytes+1024*1024-1)&~(1024*1024-1);
  SceUID memblock=sceKernelAllocMemBlock("VitaSurfNetFrame",SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW,2*stride,NULL);
  if(memblock<0)return -1;
  void *base=NULL;
  if(sceKernelGetMemBlockBase(memblock,&base)<0){sceKernelFreeMemBlock(memblock);return -2;}
  uint32_t *screens[2]={(uint32_t *)base,(uint32_t *)((char *)base+stride)};
  for(int i=0;i<2;i++){fb=screens[i];draw(i,0);}
  SceDisplayFrameBuf frame={0};
  frame.size=sizeof(frame);
  frame.pitch=W;
  frame.pixelformat=SCE_DISPLAY_PIXELFORMAT_A8B8G8R8;
  frame.width=W;
  frame.height=H;
  sceCtrlSetSamplingMode(SCE_CTRL_MODE_DIGITAL);
  int active=0;unsigned prev=0;
  for(unsigned frames=0;;frames++){
    SceCtrlData pad={0};
    sceCtrlPeekBufferPositive(0,&pad,1);
    if(pad.buttons&SCE_CTRL_START)break;
    if((pad.buttons&SCE_CTRL_CROSS)&&!(prev&SCE_CTRL_CROSS)){
      active=1-active;
      fb=screens[active];stage=1;draw(active,frames);
      frame.base=fb;
      sceDisplaySetFrameBuf(&frame,SCE_DISPLAY_SETBUF_NEXTFRAME);
      sceDisplayWaitVblankStart();
      do_fetch();
      active=1-active;
      fb=screens[active];
      draw(active,frames);
    }
    prev=pad.buttons;
    frame.base=screens[active];
    sceDisplaySetFrameBuf(&frame,SCE_DISPLAY_SETBUF_NEXTFRAME);
    sceDisplayWaitVblankStart();
  }
  network_shutdown();
  sceKernelFreeMemBlock(memblock);
  return 0;
}
