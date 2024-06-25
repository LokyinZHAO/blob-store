cmake_minimum_required(VERSION 3.15)
project(blob_store)

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