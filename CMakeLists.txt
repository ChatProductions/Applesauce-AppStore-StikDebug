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

# Browser navigation hardware test: clickable address bar and Vita system IME.
# Uses GXM-backed presentation while the native keyboard dialog is active.
if(EXISTS "${CMAKE_SOURCE_DIR}/vendor/lexbor_single.h")
  add_executable(vitasurf_address src/address_test.c)
  target_include_directories(vitasurf_address PRIVATE vendor)
  # The CPU rasterizer previously rebuilt 960x544 pixels every frame at -O0.
  target_compile_options(vitasurf_address PRIVATE -O2)
  target_link_libraries(vitasurf_address
    SceDisplay_stub SceCtrl_stub SceTouch_stub SceKernelThreadMgr_stub SceSysmem_stub
    SceNet_stub SceNetCtl_stub SceHttp_stub SceSsl_stub SceSysmodule_stub
    SceGxm_stub SceAppUtil_stub SceCommonDialog_stub SceIme_stub m)
  vita_create_self(eboot-address.bin vitasurf_address)
  vita_create_vpk(VitaSurf-AddressBar-Test.vpk VTSF00005 eboot-address.bin
    VERSION 00.02 NAME "VitaSurf Address Bar")
endif()

# Controlled DOM + JavaScript integration probe. The known-good address bar
# application remains untouched; no external website scripts run in this build.
if(EXISTS "${CMAKE_SOURCE_DIR}/vendor/quickjs/quickjs.c"
   AND EXISTS "${CMAKE_SOURCE_DIR}/vendor/lexbor_single.h")
  add_executable(vitasurf_domjs
    src/domjs_test.c
    vendor/quickjs/quickjs.c vendor/quickjs/libregexp.c
    vendor/quickjs/libunicode.c vendor/quickjs/cutils.c vendor/quickjs/dtoa.c)
  target_include_directories(vitasurf_domjs PRIVATE vendor vendor/quickjs)
  target_compile_definitions(vitasurf_domjs PRIVATE _GNU_SOURCE CONFIG_VERSION="2025-09-13")
  target_compile_options(vitasurf_domjs PRIVATE -O2)
  # Keep the large QuickJS engine at -O0: the browser UI is optimized,
  # but optimizing the engine's enormous source makes CI much slower.
  set_source_files_properties(vendor/quickjs/quickjs.c PROPERTIES COMPILE_FLAGS "-O0")
  target_link_libraries(vitasurf_domjs
    SceDisplay_stub SceCtrl_stub SceTouch_stub SceKernelThreadMgr_stub SceSysmem_stub
    SceNet_stub SceNetCtl_stub SceHttp_stub SceSsl_stub SceSysmodule_stub
    SceGxm_stub SceAppUtil_stub SceCommonDialog_stub SceIme_stub m)
  vita_create_self(eboot-domjs.bin vitasurf_domjs)
  vita_create_vpk(VitaSurf-DOMJS-Test.vpk VTSF00006 eboot-domjs.bin
    VERSION 00.01 NAME "VitaSurf DOM JS Test")
endif()
