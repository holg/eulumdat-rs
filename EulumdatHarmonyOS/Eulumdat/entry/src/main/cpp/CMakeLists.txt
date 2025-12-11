# CMakeLists.txt for Eulumdat HarmonyOS Native Module
#
# This builds the NAPI bridge that connects ArkTS to the Rust FFI library.

cmake_minimum_required(VERSION 3.4.1)
project(eulumdat_napi)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})
set(EULUMDAT_LIBS_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../../libs/${OHOS_ARCH})

# Include directories
include_directories(${NATIVERENDER_ROOT_PATH}
                    ${NATIVERENDER_ROOT_PATH}/include)

# Link directories
link_directories(${EULUMDAT_LIBS_DIR})

# Build the NAPI bridge
add_library(eulumdat_napi SHARED eulumdat_napi.cpp)

# Link against HarmonyOS NAPI and our Rust library
target_link_libraries(eulumdat_napi PUBLIC libace_napi.z.so)
target_link_libraries(eulumdat_napi PUBLIC eulumdat_harmonyos_ffi)
