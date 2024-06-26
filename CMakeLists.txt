cmake_minimum_required(VERSION 3.15)
project(blob_store)

option(ENABLE_LTO "Enable cross language linking time optimization" OFF)
if(ENABLE_LTO)
  include(CheckIPOSupported)
  check_ipo_supported(RESULT supported OUTPUT error)
  if(supported)
      message(STATUS "IPO / LTO enabled")
      set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
      add_link_options(-fuse-ld=lld)
  else()
      message(STATUS "IPO / LTO not supported: <${error}>")
  endif()
endif()

if(ENABLE_LTO)
    set(RUST_FLAGS "-Clinker-plugin-lto" "-Clinker=clang" "-Clink-arg=-fuse-ld=lld")
endif()


set(CMAKE_CXX_STANDARD 17)
if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build)
    set(TARGET_DIR "debug")
else ()
    set(CARGO_CMD cargo build --release)
    set(TARGET_DIR "release")
endif ()

set(CRATE_NAME "blob_store")
set(CARGO_TARGET ${CMAKE_CURRENT_SOURCE_DIR}/target)
set(CXX_BRIDGE ${CARGO_TARGET}/cxxbridge/blob-store)

set(RUST_PART_LIB "${CARGO_TARGET}/${TARGET_DIR}/lib${CRATE_NAME}.a")

set(RUST_PART_CXX 
    ${CXX_BRIDGE}/src/ffi/local_file_system.rs.cc
    ${CXX_BRIDGE}/src/ffi/sqlite.rs.cc
    ${CXX_BRIDGE}/src/ffi/memmap.rs.cc
)

add_library(blob-store-rs STATIC ${RUST_PART_CXX})
target_include_directories(blob-store-rs PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include/)
add_custom_command(
    OUTPUT ${RUST_PART_CXX} ${RUST_PART_LIB}
    COMMAND ${CARGO_CMD}
    COMMAND ${CMAKE_COMMAND} -E copy ${CXX_BRIDGE}/src/ffi/local_file_system.rs.h ${CMAKE_CURRENT_SOURCE_DIR}/include/
    COMMAND ${CMAKE_COMMAND} -E copy ${CXX_BRIDGE}/src/ffi/sqlite.rs.h ${CMAKE_CURRENT_SOURCE_DIR}/include/
    COMMAND ${CMAKE_COMMAND} -E copy ${CXX_BRIDGE}/src/ffi/memmap.rs.h ${CMAKE_CURRENT_SOURCE_DIR}/include/
    DEPENDS 
        ${CMAKE_CURRENT_SOURCE_DIR}/build.rs
        ${CMAKE_CURRENT_SOURCE_DIR}/src/ffi/local_file_system.rs
        ${CMAKE_CURRENT_SOURCE_DIR}/src/ffi/sqlite.rs
        ${CMAKE_CURRENT_SOURCE_DIR}/src/ffi/memmap.rs
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(blob-store-rs ${RUST_PART_LIB})

add_executable(example example.cpp)
target_link_libraries(example blob-store-rs)

add_executable(bench bench.cpp)
target_link_libraries(bench blob-store-rs)