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
#include "lexbor_single.h"
#include "quickjs.h"
#include <psp2/gxm.h>
#include <psp2/apputil.h>
#include <psp2/common_dialog.h>
#include <psp2/ime_dialog.h>
#include <psp2/touch.h>

#define W 960
#define H 544
#define PITCH 1024
#define URL_CAP 256
#define IME_URL_CAP 220
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
/* Dedicated 5x7 glyphs for Turkish uppercase diacritics. */
static uint8_t glyph(unsigned int c,int y) {
  static const uint8_t tr[6][7]={
    {4,0,31,4,4,4,31},    /* 0x80: İ (dotted capital I) */
    {15,16,16,14,1,30,4},/* 0x81: Ş */
    {4,14,17,23,17,17,14},/* 0x82: Ğ */
    {14,17,16,16,17,14,4},/* 0x83: Ç */
    {10,14,17,17,17,17,14},/* 0x84: Ö */
    {10,17,17,17,17,17,14} /* 0x85: Ü */
  };
  if(c>=0x80&&c<=0x85)return tr[c-0x80][y];
  if(c>='a'&&c<='z')c=(char)(c-32);
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
  if(c=='/') return (y==0?1:y==1?2:y==2?4:y==3?8:y==4?16:0);
  if(c=='?') return (y==0?14:y==1?17:y==2?1:y==3?2:y==4?4:y==5?0:4);
  if(c=='_') return y==6?31:0;
  if(c=='=') return y==2||y==4?31:0;
  if(c=='&') { static const uint8_t t[]={4,10,10,4,21,18,13}; return t[y]; }
  if(c=='%') { static const uint8_t t[]={17,18,4,4,8,19,17}; return t[y]; }
  if(c=='#') return y==2||y==4?31:10;
  if(c=='+') return y==3?31:(y==2||y==4?4:0);
  if(c=='@') { static const uint8_t t[]={14,17,23,21,23,16,14}; return t[y]; }
  if(c==' ')return 0;
  if(c==':') return y==2||y==5?4:0;
  if(c=='.') return y==6?4:0;
  if(c=='-') return y==3?31:0;
  return 0;
}
static void rect(int x,int y,int w,int h,uint32_t color) {
  for(int j=y;j<y+h&&j<H;j++) for(int i=x;i<x+w&&i<W;i++) if(i>=0&&j>=0) fb[j*PITCH+i]=color;
}
static void label(int x,int y,const char *s,uint32_t color,int scale) {
  while(*s){ for(int j=0;j<7;j++){uint8_t bits=glyph((unsigned char)*s,j);for(int i=0;i<5;i++)if(bits&(1<<(4-i)))rect(x+i*scale,y+j*scale,scale,scale,color);} x+=6*scale;s++; }
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
static char lines[64][78];
static int line_total=0;
static int poscol=0;
static int scroll=0;
static int dom_elements=0;
static int dom_links=0;
static int parser_status=-1;
static char current_url[URL_CAP]="https://example.com/";
static int edit_error=0;
static int ime_active=0;
static int domjs_result=0;
static int domjs_script_count=0;
static lxb_html_document_t *bridge_document=NULL;
static uint16_t ime_initial[IME_URL_CAP+1]={0};
static uint16_t ime_input[IME_URL_CAP+1]={0};

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

static void line_break(void) {
  if(line_total>=64)return;
  if(poscol>0) {
    lines[line_total][poscol]=0;
    line_total++;
    poscol=0;
  }
}

/* Decode UTF-8 before layout: a multibyte Turkish letter occupies one cell. */
static int turkish_page=0;
static uint32_t next_codepoint(const unsigned char *src,size_t n,size_t *i){
  unsigned c=src[(*i)++];
  if(c<0x80)return c;
  int more=(c>=0xC2&&c<=0xDF)?1:
           (c>=0xE0&&c<=0xEF)?2:
           (c>=0xF0&&c<=0xF4)?3:0;
  if(!more || *i+(size_t)more>n)return '?';
  uint32_t cp=c&((1U<<(6-more))-1U);
  for(int k=0;k<more;k++){
    unsigned v=src[(*i)++];
    if((v&0xC0)!=0x80)return '?';
    cp=(cp<<6)|(v&0x3F);
  }
  if(cp>0x10FFFF||(cp>=0xD800&&cp<=0xDFFF))return '?';
  return cp;
}
static unsigned int output_glyph(uint32_t cp){
  if(cp=='i' && turkish_page)return 0x80;
  if(cp==0x0130)return 0x80;        /* İ / i with dot */
  if(cp==0x0131)return 'I';          /* dotless ı */
  if(cp==0x015E||cp==0x015F)return 0x81; /* Ş ş */
  if(cp==0x011E||cp==0x011F)return 0x82; /* Ğ ğ */
  if(cp==0x00C7||cp==0x00E7)return 0x83; /* Ç ç */
  if(cp==0x00D6||cp==0x00F6)return 0x84; /* Ö ö */
  if(cp==0x00DC||cp==0x00FC)return 0x85; /* Ü ü */
  if(cp==0x00A0||cp==0x202F)return ' ';
  if(cp==0x2013||cp==0x2014||cp==0x2212)return '-';
  if(cp==0x2018||cp==0x2019)return 39;
  if(cp==0x201C||cp==0x201D)return 34;
  if(cp==0x2026)return '.';
  if(cp==0x00C2||cp==0x00E2||cp==0x00C1||cp==0x00E1||cp==0x00C0||cp==0x00E0) return 'A';
  if(cp==0x00C9||cp==0x00E9||cp==0x00C8||cp==0x00E8||cp==0x00CA||cp==0x00EA) return 'E';
  if(cp==0x00CE||cp==0x00EE||cp==0x00CD||cp==0x00ED) return 'I';
  if(cp==0x00D4||cp==0x00F4||cp==0x00D3||cp==0x00F3) return 'O';
  if(cp==0x00DB||cp==0x00FB||cp==0x00DA||cp==0x00FA) return 'U';
  if(cp>=97&&cp<=122)cp-=32;
  if(cp>=32&&cp<=126)return (unsigned int)cp;
  return '?'; /* Explicit fallback instead of silently deleting text. */
}
static int has_turkish_utf8(const char *s,int len){
  for(int i=0;i+1<len;i++){
    unsigned char a=(unsigned char)s[i],b=(unsigned char)s[i+1];
    if(a==0xC4 && (b==0xB0||b==0xB1||b==0x9E||b==0x9F))return 1;
    if(a==0xC5 && (b==0x9E||b==0x9F))return 1;
    if(a==0xC3 && (b==0x87||b==0xA7||b==0x96||b==0xB6||b==0x9C||b==0xBC))return 1;
  }
  return 0;
}
static void append_visible(const lxb_char_t *text,size_t size){
  if(!text)return;
  for(size_t i=0;i<size && line_total<64;){
    uint32_t cp=next_codepoint((const unsigned char *)text,size,&i);
    if(cp==10||cp==13||cp==9)cp=' ';
    unsigned int c=output_glyph(cp);
    if(c==' ' && (poscol==0 || lines[line_total][poscol-1]==' '))continue;
    if(poscol>=63 && c==' '){line_break();continue;}
    if(poscol>=66)line_break();
    if(line_total>=64)break;
    lines[line_total][poscol++]=(char)c;
    lines[line_total][poscol]=0;
  }
}
static void walk_dom(lxb_dom_node_t *node,int depth){
  if(!node||depth>80||line_total>=63)return;
  lxb_tag_id_t tag=lxb_dom_node_tag_id(node);
  if(node->type==LXB_DOM_NODE_TYPE_ELEMENT){
    dom_elements++;
    if(tag==LXB_TAG_A)dom_links++;
    if(tag==LXB_TAG_SCRIPT || tag==LXB_TAG_STYLE || tag==LXB_TAG_NOSCRIPT)return;
    if(tag==LXB_TAG_BR || tag==LXB_TAG_P || tag==LXB_TAG_H1 || tag==LXB_TAG_H2 || tag==LXB_TAG_LI)
      line_break();
  }
  if(node->type==LXB_DOM_NODE_TYPE_TEXT){
    size_t len=0;
    lxb_char_t *part=lxb_dom_node_text_content(node,&len);
    append_visible(part,len);
  }
  for(lxb_dom_node_t *child=lxb_dom_node_first_child(node);child;child=lxb_dom_node_next(child))
    walk_dom(child,depth+1);
  if(node->type==LXB_DOM_NODE_TYPE_ELEMENT &&
    (tag==LXB_TAG_P || tag==LXB_TAG_H1 || tag==LXB_TAG_H2 || tag==LXB_TAG_LI || tag==LXB_TAG_DIV))
    line_break();
}
static void parse_response(const char *data,int len){
  html_seen=0;has_title=0;dom_links=0;dom_elements=0;line_total=0;poscol=0;scroll=0;parser_status=-1;
  memset(lines,0,sizeof(lines));strcpy(title_text,"NO TITLE");
  turkish_page=(strstr(current_url,".tr")!=NULL)||has_turkish_utf8(data,len);
  lxb_html_document_t *doc=lxb_html_document_create();
  if(!doc)return;
  lxb_status_t st=lxb_html_document_parse(doc,(const lxb_char_t*)data,(size_t)len);
  parser_status=(int)st;
  if(st==LXB_STATUS_OK){
    html_seen=1;
    size_t tlen=0;
    const lxb_char_t *title=lxb_html_document_title(doc,&tlen);
    if(title && tlen){
      int out=0;
      for(size_t i=0;i<tlen && out<(int)sizeof(title_text)-1;){
        unsigned int c=output_glyph(next_codepoint(title,tlen,&i));
        title_text[out++]=(char)c;
      }
      title_text[out]=0;
      has_title=out>0;
    }
    if(doc->body){
      walk_dom(lxb_dom_interface_node(doc->body),0);
      line_break();
    }
  }
  lxb_html_document_destroy(doc);
}
static void do_fetch(void) {
  requests++;
  stage=1;failure=0;http_code=0;received=0;html_seen=0;has_title=0;line_total=0;
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
  const char *url=current_url;
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
  stage=(failure==0 && http_code==200 && html_seen)?2:3;
}


/* Minimal, bounded QuickJS <-> Lexbor bridge for an isolated local HTML fixture.
   Real websites' untrusted scripts are deliberately not enabled in this probe. */
static JSValue bridge_get_title(JSContext *ctx,JSValueConst self,int argc,JSValueConst *argv){
  (void)self;(void)argc;(void)argv;
  if(!bridge_document)return JS_ThrowInternalError(ctx,"no active DOM");
  size_t size=0;
  const lxb_char_t *val=lxb_html_document_title(bridge_document,&size);
  return JS_NewStringLen(ctx,val?(const char*)val:"",val?size:0);
}
static JSValue bridge_set_title(JSContext *ctx,JSValueConst self,int argc,JSValueConst *argv){
  (void)self;
  if(!bridge_document || argc<1)return JS_ThrowTypeError(ctx,"missing DOM or title");
  size_t len=0;
  const char *text=JS_ToCStringLen(ctx,&len,argv[0]);
  if(!text)return JS_EXCEPTION;
  lxb_status_t st=lxb_html_document_title_set(bridge_document,(const lxb_char_t*)text,len);
  JS_FreeCString(ctx,text);
  if(st!=LXB_STATUS_OK)return JS_ThrowInternalError(ctx,"Lexbor title update failed");
  return JS_UNDEFINED;
}
static JSValue bridge_get_body(JSContext *ctx,JSValueConst self,int argc,JSValueConst *argv){
  (void)self;(void)argc;(void)argv;
  if(!bridge_document || !bridge_document->body)return JS_ThrowInternalError(ctx,"no document body");
  size_t len=0;
  lxb_char_t *content=lxb_dom_node_text_content(lxb_dom_interface_node(bridge_document->body),&len);
  return JS_NewStringLen(ctx,content?(const char*)content:"",content?len:0);
}
static JSValue bridge_set_body(JSContext *ctx,JSValueConst self,int argc,JSValueConst *argv){
  (void)self;
  if(!bridge_document || !bridge_document->body || argc<1)return JS_ThrowTypeError(ctx,"missing body");
  size_t len=0;
  const char *text=JS_ToCStringLen(ctx,&len,argv[0]);
  if(!text)return JS_EXCEPTION;
  lxb_status_t st=lxb_dom_node_text_content_set(lxb_dom_interface_node(bridge_document->body),(const lxb_char_t*)text,len);
  JS_FreeCString(ctx,text);
  if(st!=LXB_STATUS_OK)return JS_ThrowInternalError(ctx,"Lexbor body update failed");
  return JS_UNDEFINED;
}
static int add_property(JSContext *ctx,JSValue obj,const char *name,JSCFunction getter,JSCFunction setter){
  JSAtom atom=JS_NewAtom(ctx,name);
  if(atom==JS_ATOM_NULL)return -1;
  JSValue get=JS_NewCFunction(ctx,getter,name,0);
  JSValue set=JS_NewCFunction(ctx,setter,name,1);
  int r=JS_DefinePropertyGetSet(ctx,obj,atom,get,set,JS_PROP_CONFIGURABLE|JS_PROP_ENUMERABLE);
  JS_FreeAtom(ctx,atom);
  return r;
}
static lxb_dom_node_t *find_inline_script(lxb_dom_node_t *root,int depth){
  if(!root||depth>80)return NULL;
  if(root->type==LXB_DOM_NODE_TYPE_ELEMENT && lxb_dom_node_tag_id(root)==LXB_TAG_SCRIPT)return root;
  for(lxb_dom_node_t *child=lxb_dom_node_first_child(root);child;child=lxb_dom_node_next(child)){
    lxb_dom_node_t *found=find_inline_script(child,depth+1);
    if(found)return found;
  }
  return NULL;
}
static void run_domjs_fixture(void){
  static const char html[]=
    "<!doctype html><html><head><title>BEFORE JS</title></head>"
    "<body><p>ORIGINAL HTML CONTENT</p>"
    "<script>document.title='JS DOM SUCCESS';"
    "document.body.textContent='QUICKJS UPDATED THE LEXBOR DOM ON VITA';"
    "</script></body></html>";
  domjs_result=-1;domjs_script_count=0;
  strcpy(current_url,"vitasurf://domjs-test");
  stage=3;failure=0;http_code=0;received=(int)sizeof(html)-1;
  line_total=0;scroll=0;dom_links=0;dom_elements=0;
  lxb_html_document_t *doc=lxb_html_document_create();
  if(!doc)return;
  if(lxb_html_document_parse(doc,(const lxb_char_t*)html,strlen(html))!=LXB_STATUS_OK){
    lxb_html_document_destroy(doc);return;
  }
  bridge_document=doc;
  lxb_dom_node_t *script=find_inline_script(lxb_dom_interface_node(doc),0);
  size_t len=0;
  lxb_char_t *script_data=script?lxb_dom_node_text_content(script,&len):NULL;
  char *source=NULL;
  if(script_data&&len>0&&len<4096){
    source=(char*)malloc(len+1);
    if(source){memcpy(source,script_data,len);source[len]=0;}
  }
  if(source){
    JSRuntime *rt=JS_NewRuntime();
    if(rt){
      JS_SetMemoryLimit(rt,16*1024*1024);
      JS_SetMaxStackSize(rt,512*1024);
      JSContext *ctx=JS_NewContext(rt);
      if(ctx){
        JSValue global=JS_GetGlobalObject(ctx);
        JSValue d=JS_NewObject(ctx);
        JSValue b=JS_NewObject(ctx);
        int ok=add_property(ctx,d,"title",bridge_get_title,bridge_set_title)>=0;
        ok=ok&&add_property(ctx,b,"textContent",bridge_get_body,bridge_set_body)>=0;
        if(ok)ok=JS_SetPropertyStr(ctx,d,"body",b)>=0;
        else JS_FreeValue(ctx,b);
        if(ok)ok=JS_SetPropertyStr(ctx,global,"document",d)>=0;
        else JS_FreeValue(ctx,d);
        if(ok){
          JSValue value=JS_Eval(ctx,source,len,"inline-script",JS_EVAL_TYPE_GLOBAL);
          if(!JS_IsException(value))domjs_script_count=1;
          JS_FreeValue(ctx,value);
        }
        JS_FreeValue(ctx,global);
        JS_FreeContext(ctx);
      }
      JS_FreeRuntime(rt);
    }
  }
  free(source);
  /* This verifies that JavaScript really mutated the same DOM used by the UI. */
  size_t title_len=0,body_len=0;
  const lxb_char_t *title=lxb_html_document_title(doc,&title_len);
  lxb_char_t *body=doc->body?lxb_dom_node_text_content(lxb_dom_interface_node(doc->body),&body_len):NULL;
  if(domjs_script_count==1 && title && title_len==14 &&
     memcmp(title,"JS DOM SUCCESS",14)==0 && body && body_len==38 &&
     memcmp(body,"QUICKJS UPDATED THE LEXBOR DOM ON VITA",38)==0){
    domjs_result=1;
  }
  /* Reuse the real page renderer to display the updated DOM. */
  parse_response(html,(int)strlen(html));
  /* parse_response rebuilt a new document; show the modified one instead. */
  line_total=0;poscol=0;scroll=0;dom_links=0;dom_elements=0;
  memset(lines,0,sizeof(lines));
  if(doc->body){walk_dom(lxb_dom_interface_node(doc->body),0);line_break();}
  int n=0;
  for(size_t i=0;i<title_len && n<(int)sizeof(title_text)-1;i++)
    title_text[n++]=(char)title[i];
  title_text[n]=0;has_title=n>0;html_seen=1;stage=2;
  bridge_document=NULL;
  lxb_html_document_destroy(doc);
}

/* The Vita native IME dialog requires a GXM-backed render target. */
typedef struct {
  void *data;
  SceGxmSyncObject *sync;
  SceGxmColorSurface surf;
  SceUID uid;
} DisplayBuffer;
static DisplayBuffer dbuf[2];
static unsigned front_index=0,back_index=0;
static int gxm_ready=0,apputil_ready=0,ime_module_ready=0;

#define ALIGN_BUF(x,a) (((x)+((a)-1))&~((a)-1))
static void display_callback(const void *userdata) {
  SceDisplayFrameBuf frame={0};
  frame.size=sizeof(frame);
  frame.base=*(void *const *)userdata;
  frame.pitch=PITCH;
  frame.pixelformat=SCE_DISPLAY_PIXELFORMAT_A8B8G8R8;
  frame.width=W;
  frame.height=H;
  sceDisplaySetFrameBuf(&frame,SCE_DISPLAY_SETBUF_NEXTFRAME);
}
static int init_gxm(void){
  int r=sceGxmInitialize(&(SceGxmInitializeParams){
    0,1,display_callback,sizeof(void *),SCE_GXM_DEFAULT_PARAMETER_BUFFER_SIZE});
  if(r<0)return r;
  gxm_ready=1;
  const unsigned sz=ALIGN_BUF(4*PITCH*H,256*1024);
  for(int i=0;i<2;i++){
    dbuf[i].uid=sceKernelAllocMemBlock("VitaSurfBrowserFrame",SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW,sz,NULL);
    if(dbuf[i].uid<0)return dbuf[i].uid;
    r=sceKernelGetMemBlockBase(dbuf[i].uid,&dbuf[i].data);
    if(r<0)return r;
    r=sceGxmMapMemory(dbuf[i].data,sz,SCE_GXM_MEMORY_ATTRIB_READ|SCE_GXM_MEMORY_ATTRIB_WRITE);
    if(r<0)return r;
    r=sceGxmColorSurfaceInit(&dbuf[i].surf,SCE_GXM_COLOR_FORMAT_A8B8G8R8,
      SCE_GXM_COLOR_SURFACE_LINEAR,SCE_GXM_COLOR_SURFACE_SCALE_NONE,
      SCE_GXM_OUTPUT_REGISTER_SIZE_32BIT,W,H,PITCH,dbuf[i].data);
    if(r<0)return r;
    r=sceGxmSyncObjectCreate(&dbuf[i].sync);
    if(r<0)return r;
  }
  return 0;
}
static void swap_gxm(void){
  sceGxmPadHeartbeat(&dbuf[back_index].surf,dbuf[back_index].sync);
  sceGxmDisplayQueueAddEntry(dbuf[front_index].sync,dbuf[back_index].sync,&dbuf[back_index].data);
  front_index=back_index;
  back_index=(back_index+1)%2;
}
static void shutdown_gxm(void){
  if(!gxm_ready)return;
  sceGxmDisplayQueueFinish();
  for(int i=0;i<2;i++){
    if(dbuf[i].sync){sceGxmSyncObjectDestroy(dbuf[i].sync);dbuf[i].sync=NULL;}
    if(dbuf[i].data){sceGxmUnmapMemory(dbuf[i].data);dbuf[i].data=NULL;}
    if(dbuf[i].uid>0){sceKernelFreeMemBlock(dbuf[i].uid);dbuf[i].uid=0;}
  }
  sceGxmTerminate();gxm_ready=0;
}
static int validate_address(const uint16_t *utf16,char *out,size_t size){
  char text[IME_URL_CAP+1]={0};
  size_t n=0;
  while(n<IME_URL_CAP && utf16[n]){
    if(utf16[n]>127 || utf16[n]<32)return -1; /* ASCII URL input in this test */
    text[n]=(char)utf16[n];
    n++;
  }
  text[n]=0;
  char *start=text;
  while(*start==' ')start++;
  char *end=start+strlen(start);
  while(end>start && end[-1]==' ')*--end=0;
  if(!*start)return -2;
  for(char *p=start;*p;p++)if((unsigned char)*p<=32)return -3;
  const char *prefix="";
  if(strncmp(start,"https://",8)==0 || strncmp(start,"http://",7)==0){
    /* already a supported scheme */
  }else{
    if(strstr(start,"://"))return -4;
    prefix="https://";
  }
  if(strlen(prefix)+strlen(start)>=size)return -5;
  snprintf(out,size,"%s%s",prefix,start);
  const char *host=strstr(out,"://");
  if(!host || !host[3] || host[3]=='/')return -6;
  return 0;
}
static void open_url_keyboard(void){
  memset(ime_initial,0,sizeof(ime_initial));
  memset(ime_input,0,sizeof(ime_input));
  for(int i=0;i<IME_URL_CAP && current_url[i];i++)ime_initial[i]=(uint16_t)(unsigned char)current_url[i];
  SceImeDialogParam p;
  sceImeDialogParamInit(&p);
  p.type=SCE_IME_TYPE_URL;
  p.option=SCE_IME_OPTION_NO_AUTO_CAPITALIZATION;
  p.dialogMode=SCE_IME_DIALOG_DIALOG_MODE_WITH_CANCEL;
  p.textBoxMode=SCE_IME_DIALOG_TEXTBOX_MODE_WITH_CLEAR;
  p.title=u"Enter website address";
  p.maxTextLength=IME_URL_CAP;
  p.initialText=ime_initial;
  p.inputTextBuffer=ime_input;
  p.enterLabel=SCE_IME_ENTER_LABEL_GO;
  int r=sceImeDialogInit(&p);
  if(r<0){edit_error=r;ime_active=0;}
  else{edit_error=0;ime_active=1;}
}
static void handle_ime_result(void){
  if(!ime_active)return;
  SceCommonDialogStatus status=sceImeDialogGetStatus();
  if(status!=SCE_COMMON_DIALOG_STATUS_FINISHED)return;
  SceImeDialogResult result={0};
  int r=sceImeDialogGetResult(&result);
  sceImeDialogTerm();
  ime_active=0;
  if(r<0){edit_error=r;return;}
  if(result.button==SCE_IME_DIALOG_BUTTON_ENTER){
    char url[URL_CAP]={0};
    int ok=validate_address(ime_input,url,sizeof(url));
    if(ok<0){edit_error=ok;return;}
    strcpy(current_url,url);
    edit_error=0;
    do_fetch();
  }
}
static void draw(int mode,unsigned frames){
  rect(0,0,W,H,0xfff4f3ee);
  rect(0,0,W,50,0xff202b42);
  label(18,12,"VITASURF",0xffffffff,3);
  label(570,16,"TRIANGLE URL SQUARE JS",0xffc6e0f0,2);
  rect(12,58,W-24,49,0xffdce5ea);
  char bar[74]={0};
  snprintf(bar,sizeof(bar),"%s",current_url);
  if(strlen(current_url)>72){bar[69]='.';bar[70]='.';bar[71]=0;}
  label(26,73,bar,0xff182638,2);
  if(stage==0){
    label(50,180,"TOUCH ADDRESS OR PRESS TRIANGLE",0xff182638,3);
    label(50,240,"ENTER A WEB ADDRESS AND GO",0xff182638,3);
  }else if(stage==1){
    label(50,180,"LOADING",0xff182638,4);
  }else if(stage==3){
    label(50,165,"PAGE LOAD FAILED",0xffbf3947,4);
    char info[72];
    snprintf(info,sizeof(info),"HTTP %d ERROR %08X",http_code,(unsigned)failure);
    label(50,230,info,0xff182638,2);
    snprintf(info,sizeof(info),"NETWORK PHASE %d",phase);
    label(50,267,info,0xff182638,2);
  }else{
    label(40,117,title_text,0xff172e46,3);
    rect(35,163,W-70,2,0xffb2c1ca);
    int shown=0;
    for(int i=scroll;i<line_total&&shown<12;i++,shown++)
      label(45,179+shown*24,lines[i],0xff293846,2);
  }
  if(edit_error){
    char info[80];
    snprintf(info,sizeof(info),"KEYBOARD ERROR %08X",(unsigned)edit_error);
    label(50,450,info,0xffbf3947,2);
  }
  if(domjs_result!=0)label(50,447,domjs_result==1?"DOM JS PASS":"DOM JS FAIL",
    domjs_result==1?0xff1f947e:0xffbf3947,2);
  rect(0,487,W,57,0xff202b42);
  char info[90];
  snprintf(info,sizeof(info),"HTTP %d BYTES %d DOM %d LINKS %d",http_code,received,dom_elements,dom_links);
  label(18,492,info,0xffffffff,2);
  label(18,520,"SQUARE DOMJS  TRIANGLE URL  CROSS RELOAD  START EXIT",0xffe0e7ed,2);
  (void)mode;(void)frames;
}
int main(void){
  for(int i=0;i<2;i++)dbuf[i].uid=-1;
  int r=sceAppUtilInit(&(SceAppUtilInitParam){0},&(SceAppUtilBootParam){0});
  if(r>=0){
    apputil_ready=1;
    sceCommonDialogSetConfigParam(&(SceCommonDialogConfigParam){0});
  }
  r=sceSysmoduleLoadModule(SCE_SYSMODULE_IME);
  if(r>=0)ime_module_ready=1;
  r=init_gxm();
  if(r<0){shutdown_gxm();if(ime_module_ready)sceSysmoduleUnloadModule(SCE_SYSMODULE_IME);
    if(apputil_ready)sceAppUtilShutdown();return r;}
  sceCtrlSetSamplingMode(SCE_CTRL_MODE_DIGITAL);
  sceTouchSetSamplingState(SCE_TOUCH_PORT_FRONT,SCE_TOUCH_SAMPLING_STATE_START);
  unsigned prev=0;
  int was_touching=0;
  int repaint_remaining=2; /* Only redraw on changes; 2 frames refresh both buffers. */
  for(unsigned frames=0;;frames++){
    SceCtrlData pad={0};
    sceCtrlPeekBufferPositive(0,&pad,1);
    SceTouchData touch={0};
    sceTouchPeek(SCE_TOUCH_PORT_FRONT,&touch,1);
    int touching=touch.reportNum>0;
    int touch_bar=touching&&!was_touching&&
      touch.report[0].y/2>=57&&touch.report[0].y/2<=111;
    was_touching=touching;
    if(ime_active){
      handle_ime_result();
      if(!ime_active)repaint_remaining=2; /* Remove residual IME pixels. */
    }else{
      if((pad.buttons&SCE_CTRL_START)&&!(prev&SCE_CTRL_START))break;
      if(((pad.buttons&SCE_CTRL_TRIANGLE)&&!(prev&SCE_CTRL_TRIANGLE))||touch_bar){
        if(apputil_ready)open_url_keyboard();
        else edit_error=-100;
        repaint_remaining=2;
      }else if((pad.buttons&SCE_CTRL_SQUARE)&&!(prev&SCE_CTRL_SQUARE)){
        run_domjs_fixture();
        repaint_remaining=2;
      }else if((pad.buttons&SCE_CTRL_CROSS)&&!(prev&SCE_CTRL_CROSS)){
        domjs_result=0;
        do_fetch();
        repaint_remaining=2;
      }else if((pad.buttons&SCE_CTRL_DOWN)&&!(prev&SCE_CTRL_DOWN)){
        if(scroll+12<line_total){scroll++;repaint_remaining=2;}
      }else if((pad.buttons&SCE_CTRL_UP)&&!(prev&SCE_CTRL_UP)){
        if(scroll>0){scroll--;repaint_remaining=2;}
      }
    }
    prev=pad.buttons;
    fb=(uint32_t*)dbuf[back_index].data;
    if(ime_active){
      /* The system keyboard animates: it needs a fresh GXM render target. */
      draw(0,frames);
      SceCommonDialogUpdateParam update={0};
      update.renderTarget.colorFormat=SCE_GXM_COLOR_FORMAT_A8B8G8R8;
      update.renderTarget.surfaceType=SCE_GXM_COLOR_SURFACE_LINEAR;
      update.renderTarget.width=W;
      update.renderTarget.height=H;
      update.renderTarget.strideInPixels=PITCH;
      update.renderTarget.depthSurfaceData=NULL;
      update.renderTarget.colorSurfaceData=fb;
      update.displaySyncObject=dbuf[back_index].sync;
      sceCommonDialogUpdate(&update);
    }else if(repaint_remaining>0){
      /* Stop touching the displayed framebuffer. Draw only the safe back
         buffer, then present; on the next frame redraw the other buffer. */
      sceGxmDisplayQueueFinish();
      draw(0,frames);
      repaint_remaining--;
    }
    /* Keep 60 Hz scanout / GXM queue submissions without per-frame CPU rasterization. */
    swap_gxm();
    sceDisplayWaitVblankStart();
  }
  if(ime_active){sceImeDialogAbort();sceImeDialogTerm();}
  sceTouchSetSamplingState(SCE_TOUCH_PORT_FRONT,SCE_TOUCH_SAMPLING_STATE_STOP);
  network_shutdown();
  shutdown_gxm();
  if(ime_module_ready)sceSysmoduleUnloadModule(SCE_SYSMODULE_IME);
  if(apputil_ready)sceAppUtilShutdown();
  return 0;
}
