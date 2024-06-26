cmake_minimum_required(VERSION 3.15)
project(blob_store)

option(ENABLE_LTO "Enable cross language linking time optimization" ON)
if(ENABLE_LTO)
  if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    message(STATUS "using Clang compiler: ${CMAKE_CXX_COMPILER_ID}")
    include(CheckIPOSupported)
    check_ipo_supported(RESULT supported OUTPUT error)
    if(supported)
      message(STATUS "IPO / LTO enabled")
      set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
      add_link_options(-fuse-ld=lld)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -flto")
      set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -flto")
      set(RUST_FLAGS "-Clinker-plugin-lto -Clinker=clang -Clink-arg=-fuse-ld=lld")
      set(ENV{RUSTFLAGS} "${RUST_FLAGS}")
    else(supported)
      message(STATUS "IPO / LTO not supported: <${error}>")
    endif(supported)
  else(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    message(STATUS "IPO / LTO requires Clang compiler")
  endif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
endif(ENABLE_LTO)

set(CMAKE_CXX_STANDARD 17)
if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build)
    set(TARGET_DIR "debug")
else ()
    set(CARGO_CMD cargo build --release)
    set(TARGET_DIR "release")
endif ()

set(CRATE_NAME "blob_store")
set(CARGO_TARGET ${CMAKE_BINARY_DIR}/target)
set(CXX_BRIDGE ${CARGO_TARGET}/cxxbridge/blob-store)

set(RUST_PART_LIB "${CARGO_TARGET}/${TARGET_DIR}/lib${CRATE_NAME}.a")

set(RUST_PART_CXX 
    ${CXX_BRIDGE}/src/ffi/local_file_system.rs.cc
    ${CXX_BRIDGE}/src/ffi/memmap.rs.cc
)

set(CARGO_FEATURES "--features=default")

option(FEAT_SQLITE "Enable SQLITE backend feature" OFF)
if (FEAT_SQLITE)
    message(STATUS "Enabling SQLITE backend feature")
    list(APPEND RUST_PART_CXX  ${CXX_BRIDGE}/src/ffi/sqlite.rs.cc)
    list(APPEND CARGO_FEATURES "--features=sqlite")
endif()

add_library(blob-store-rs STATIC ${RUST_PART_CXX})
target_include_directories(blob-store-rs PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include/)
add_custom_command(
    OUTPUT ${RUST_PART_CXX} ${RUST_PART_LIB}
    # COMMAND ${CMAKE_COMMAND} -E echo "Environment variable RUST_FLAGS is $ENV{RUSTFLAGS}"
    COMMAND CARGO_TARGET_DIR=${CARGO_TARGET} ${CARGO_CMD} ${CARGO_FEATURES}
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
if(FEAT_SQLITE)
    target_compile_definitions(bench PUBLIC _ENABLE_SQLITE)
endif(FEAT_SQLITE)

target_link_libraries(bench blob-store-rs)