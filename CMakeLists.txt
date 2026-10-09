cmake_minimum_required(VERSION 3.24)
project(DrawVerse VERSION 0.1.0 LANGUAGES C CXX)

# ABI consumers use C/C++; the QML UI can be enabled independently.
option(DRAWVERSE_BUILD_UI "Build the Qt Quick / QML painting UI and Qt tests" OFF)

find_program(DRAWVERSE_CARGO_EXECUTABLE NAMES cargo REQUIRED)
find_package(Python3 3.9 REQUIRED COMPONENTS Interpreter)
find_package(Threads REQUIRED)

include(cmake/MsvcNinjaDependencies.cmake)
set(DRAWVERSE_CARGO_TARGET_DIR "${CMAKE_SOURCE_DIR}/core/target" CACHE PATH "Rust build output directory")

if(WIN32 AND NOT MSVC)
    message(FATAL_ERROR "Current Windows ABI build uses the MSVC Rust target and an MSVC C/C++ compiler. Matching MinGW target support is a later CI task.")
endif()

if(WIN32)
    set(ffi_static "${DRAWVERSE_CARGO_TARGET_DIR}/release/paint_ffi.lib")
    set(ffi_shared "${DRAWVERSE_CARGO_TARGET_DIR}/release/paint_ffi.dll")
    set(ffi_import "${DRAWVERSE_CARGO_TARGET_DIR}/release/paint_ffi.dll.lib")
elseif(APPLE)
    set(ffi_static "${DRAWVERSE_CARGO_TARGET_DIR}/release/libpaint_ffi.a")
    set(ffi_shared "${DRAWVERSE_CARGO_TARGET_DIR}/release/libpaint_ffi.dylib")
else()
    set(ffi_static "${DRAWVERSE_CARGO_TARGET_DIR}/release/libpaint_ffi.a")
    set(ffi_shared "${DRAWVERSE_CARGO_TARGET_DIR}/release/libpaint_ffi.so")
endif()

add_custom_target(drawverse_core ALL
    COMMAND "${CMAKE_COMMAND}" -E env "CARGO_TARGET_DIR=${DRAWVERSE_CARGO_TARGET_DIR}"
        "${DRAWVERSE_CARGO_EXECUTABLE}" build -p paint-core -p paint-ffi --release --locked
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/core"
    BYPRODUCTS "${ffi_static}" "${ffi_shared}" ${ffi_import}
    COMMENT "Building Rust core and C ABI static/shared libraries"
    VERBATIM)

add_custom_target(drawverse_header_check ALL
    COMMAND "${CMAKE_COMMAND}" -E env "CARGO_TARGET_DIR=${DRAWVERSE_CARGO_TARGET_DIR}"
        "${DRAWVERSE_CARGO_EXECUTABLE}" run -p paint-api-gen --locked -- --check
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/core"
    COMMENT "Checking the cbindgen header against Rust ABI definitions"
    VERBATIM)
add_dependencies(drawverse_header_check drawverse_core)

add_custom_target(generate_header
    COMMAND "${CMAKE_COMMAND}" -E env "CARGO_TARGET_DIR=${DRAWVERSE_CARGO_TARGET_DIR}"
        "${DRAWVERSE_CARGO_EXECUTABLE}" run -p paint-api-gen --locked
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/core"
    VERBATIM)

add_library(drawverse_ffi_static STATIC IMPORTED GLOBAL)
set_target_properties(drawverse_ffi_static PROPERTIES IMPORTED_LOCATION "${ffi_static}"
    INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_SOURCE_DIR}/ui/include")
add_dependencies(drawverse_ffi_static drawverse_header_check)
if(WIN32)
    # rustc --print native-static-libs for the Windows MSVC standard library.
    target_link_libraries(drawverse_ffi_static INTERFACE kernel32 ntdll userenv ws2_32 dbghelp)
else()
    target_link_libraries(drawverse_ffi_static INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)
    if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        target_link_libraries(drawverse_ffi_static INTERFACE rt util)
    endif()
endif()

add_library(drawverse_ffi_shared SHARED IMPORTED GLOBAL)
set_target_properties(drawverse_ffi_shared PROPERTIES IMPORTED_LOCATION "${ffi_shared}"
    INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_SOURCE_DIR}/ui/include")
if(WIN32)
    set_target_properties(drawverse_ffi_shared PROPERTIES IMPORTED_IMPLIB "${ffi_import}")
endif()
add_dependencies(drawverse_ffi_shared drawverse_header_check)

# Stage the DLL once, avoiding concurrent writes from consumer post-build steps.
if(WIN32)
    add_custom_target(drawverse_shared_runtime
        COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_BINARY_DIR}/bin/$<CONFIG>"
        COMMAND "${CMAKE_COMMAND}" -E copy_if_different "${ffi_shared}" "${CMAKE_BINARY_DIR}/bin/$<CONFIG>"
        VERBATIM)
    add_dependencies(drawverse_shared_runtime drawverse_header_check)
endif()

enable_testing()
foreach(linkage IN ITEMS static shared)
    foreach(language IN ITEMS c cpp)
        set(consumer "abi_${language}_${linkage}")
        add_executable(${consumer} "tests/abi/smoke.${language}")
        set_target_properties(${consumer} PROPERTIES C_STANDARD 11 C_STANDARD_REQUIRED YES C_EXTENSIONS NO
            CXX_STANDARD 20 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO
            RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")
        target_link_libraries(${consumer} PRIVATE drawverse_ffi_${linkage} Threads::Threads)
        target_compile_definitions(${consumer} PRIVATE DRAWVERSE_ABI_SMOKE_FILE="${consumer}.ora")
        if(MSVC)
            target_compile_options(${consumer} PRIVATE /W4 /WX /utf-8)
        else()
            target_compile_options(${consumer} PRIVATE -Wall -Wextra -Werror)
        endif()
        if(linkage STREQUAL "shared" AND WIN32)
            add_dependencies(${consumer} drawverse_shared_runtime)
        endif()
        add_test(NAME ${consumer} COMMAND ${consumer})
        set_tests_properties(${consumer} PROPERTIES TIMEOUT 30)
    endforeach()
endforeach()

if(DRAWVERSE_BUILD_UI)
    add_subdirectory(ui)
    get_target_property(check_qmake Qt6::qmake IMPORTED_LOCATION)
    get_filename_component(check_qt_bin "${check_qmake}" DIRECTORY)
    get_filename_component(check_qt_prefix "${check_qt_bin}" DIRECTORY)
    set(check_scope --qt-prefix "${check_qt_prefix}")
else()
    set(check_scope --core-only)
endif()

add_custom_target(check
    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tools/check.py"
        --cargo "${DRAWVERSE_CARGO_EXECUTABLE}" --cmake "${CMAKE_COMMAND}" ${check_scope}
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    VERBATIM)

add_test(NAME implemented_modules
    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tools/check.py"
        --cargo "${DRAWVERSE_CARGO_EXECUTABLE}" --cmake "${CMAKE_COMMAND}" ${check_scope})
