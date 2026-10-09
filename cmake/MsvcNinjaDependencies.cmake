# MSVC emits UTF-8 in some terminals and ANSI when started without a console.
# Both the probe and real compilation must supply UTF-8 to Ninja/CMake.
if(MSVC AND CMAKE_GENERATOR MATCHES "Ninja")
    set(drawverse_msvc_launcher "${Python3_EXECUTABLE}"
        "${CMAKE_CURRENT_LIST_DIR}/../tools/msvc_utf8.py")
    set(include_probe "${CMAKE_BINARY_DIR}/CMakeFiles/drawverse-includes-probe.c")
    file(WRITE "${include_probe}" "#include <stddef.h>\nint drawverse_include_probe;\n")
    execute_process(COMMAND ${drawverse_msvc_launcher} "${CMAKE_C_COMPILER}"
        /nologo /showIncludes /Zs /utf-8 "${include_probe}"
        RESULT_VARIABLE include_result OUTPUT_VARIABLE include_output ERROR_VARIABLE include_error
        ENCODING NONE)
    if(NOT include_result EQUAL 0)
        message(FATAL_ERROR "MSVC include dependency probe failed: ${include_output}${include_error}")
    endif()
    string(REGEX MATCH "(^|\n)([^\r\n]*: +)([A-Za-z]:[\\\\/][^\r\n]*stddef[.]h)" include_line "${include_output}")
    if(NOT include_line)
        message(FATAL_ERROR "Cannot detect MSVC /showIncludes prefix; refusing an unsafe incremental build")
    endif()
    # CMake 4.3 writes this prefix in the configure process's console code page.
    # An ASCII prefix keeps it identical when IDE/terminal build environments differ.
    set(drawverse_msvc_include_prefix "${CMAKE_MATCH_2}")
    set(CMAKE_CL_SHOWINCLUDES_PREFIX "drawverse include: ")
    foreach(language IN ITEMS C CXX)
        set(CMAKE_${language}_COMPILER_LAUNCHER ${drawverse_msvc_launcher}
            --include-prefix "${drawverse_msvc_include_prefix}"
            ${CMAKE_${language}_COMPILER_LAUNCHER})
    endforeach()
endif()
