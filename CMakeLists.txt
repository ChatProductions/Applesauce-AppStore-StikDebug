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
