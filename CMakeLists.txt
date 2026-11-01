cmake_minimum_required(VERSION 3.16)
if(NOT DEFINED CMAKE_TOOLCHAIN_FILE AND DEFINED ENV{VITASDK})
  set(CMAKE_TOOLCHAIN_FILE "$ENV{VITASDK}/share/vita.toolchain.cmake" CACHE PATH "Vita toolchain")
endif()
project(VitaSurfPOC LANGUAGES C)
if(NOT DEFINED ENV{VITASDK})
  message(FATAL_ERROR "VITASDK environment variable is required")
endif()
include("$ENV{VITASDK}/share/vita.cmake" REQUIRED)
add_executable(vitasurf src/main.c)
target_link_libraries(vitasurf SceDisplay_stub SceCtrl_stub SceKernelThreadMgr_stub SceSysmem_stub)
vita_create_self(eboot.bin vitasurf)
vita_create_vpk(VitaSurf-CacheFix.vpk VTSF00001 eboot.bin VERSION 00.03 NAME "VitaSurf Cache Fix")

# Separate JavaScript engine proof of concept. Preserve the verified display VPK.
if(EXISTS "${CMAKE_SOURCE_DIR}/vendor/quickjs/quickjs.c")
  add_executable(vitasurf_js src/js_test.c
    vendor/quickjs/quickjs.c vendor/quickjs/libregexp.c
    vendor/quickjs/libunicode.c vendor/quickjs/cutils.c
    vendor/quickjs/dtoa.c)
  target_include_directories(vitasurf_js PRIVATE vendor/quickjs)
  target_compile_definitions(vitasurf_js PRIVATE _GNU_SOURCE CONFIG_VERSION="2025-09-13")
  target_link_libraries(vitasurf_js m SceDisplay_stub SceCtrl_stub SceKernelThreadMgr_stub SceSysmem_stub)
  vita_create_self(eboot-js.bin vitasurf_js)
  vita_create_vpk(VitaSurf-JS-Test.vpk VTSF00002 eboot-js.bin VERSION 00.01 NAME "VitaSurf JS Test")
endif()

# Standalone hardware probe: HTTPS fetch + initial HTML document recognition.
# This does not replace the verified JavaScript test or imply complete HTML rendering.
add_executable(vitasurf_net src/network_test.c)
target_link_libraries(vitasurf_net
  SceDisplay_stub SceCtrl_stub SceKernelThreadMgr_stub SceSysmem_stub
  SceNet_stub SceNetCtl_stub SceHttp_stub SceSsl_stub SceSysmodule_stub)
vita_create_self(eboot-net.bin vitasurf_net)
vita_create_vpk(VitaSurf-WebLoad-Test.vpk VTSF00003 eboot-net.bin
  VERSION 00.02 NAME "VitaSurf WebLoad Test")

# Separate proof: fetch a live HTTPS document, construct a Lexbor HTML5 DOM,
# and present its actual body text in a minimal scrolling Vita display.
if(EXISTS "${CMAKE_SOURCE_DIR}/vendor/lexbor_single.h")
  add_executable(vitasurf_html src/html_test.c)
  target_include_directories(vitasurf_html PRIVATE vendor)
  target_link_libraries(vitasurf_html
    SceDisplay_stub SceCtrl_stub SceKernelThreadMgr_stub SceSysmem_stub
    SceNet_stub SceNetCtl_stub SceHttp_stub SceSsl_stub SceSysmodule_stub m)
  vita_create_self(eboot-html.bin vitasurf_html)
  vita_create_vpk(VitaSurf-HTML-Test.vpk VTSF00004 eboot-html.bin
    VERSION 00.01 NAME "VitaSurf HTML Test")
endif()
