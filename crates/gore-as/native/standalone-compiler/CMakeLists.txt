cmake_minimum_required(VERSION 3.25)

project(
    gore_as_standalone_compiler
    VERSION 0.1.0
    DESCRIPTION "Hermetic native sidecar for the GORE AngelScript compiler"
    LANGUAGES CXX
)

if(NOT WIN32 OR NOT MSVC OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
    message(FATAL_ERROR "gore-as-standalone-compiler currently supports only Windows with MSVC")
endif()

set(unreangel_core_sources
    vendor/unreangel/source/as_atomic.cpp
    vendor/unreangel/source/as_builder.cpp
    vendor/unreangel/source/as_bytecode.cpp
    vendor/unreangel/source/as_callfunc.cpp
    vendor/unreangel/source/as_compiler.cpp
    vendor/unreangel/source/as_configgroup.cpp
    vendor/unreangel/source/as_context.cpp
    vendor/unreangel/source/as_datatype.cpp
    vendor/unreangel/source/as_gc.cpp
    vendor/unreangel/source/as_generic.cpp
    vendor/unreangel/source/as_globalproperty.cpp
    vendor/unreangel/source/as_memory.cpp
    vendor/unreangel/source/as_module.cpp
    vendor/unreangel/source/as_objecttype.cpp
    vendor/unreangel/source/as_outputbuffer.cpp
    vendor/unreangel/source/as_parser.cpp
    vendor/unreangel/source/as_scriptcode.cpp
    vendor/unreangel/source/as_scriptengine.cpp
    vendor/unreangel/source/as_scriptfunction.cpp
    vendor/unreangel/source/as_scriptnode.cpp
    vendor/unreangel/source/as_scriptobject.cpp
    vendor/unreangel/source/as_string.cpp
    vendor/unreangel/source/as_string_util.cpp
    vendor/unreangel/source/as_thread.cpp
    vendor/unreangel/source/as_tokenizer.cpp
    vendor/unreangel/source/as_typeinfo.cpp
    vendor/unreangel/source/as_variablescope.cpp
    compat/src/compat_runtime.cpp
    src/core.cpp
    src/frontend_compile.cpp
    src/json.cpp
    src/module_preprocessor.cpp
    src/precompiled_data.cpp
    src/precompiled_engine.cpp
    src/precompiled_metadata.cpp
    src/profile_loader.cpp
    src/registry_profile.cpp
    src/sha256.cpp
    src/sidecar_compile.cpp
)

add_library(gore-as-unreangel-core STATIC ${unreangel_core_sources})
target_compile_features(gore-as-unreangel-core PUBLIC cxx_std_17)
target_include_directories(gore-as-unreangel-core
    PUBLIC
        "${CMAKE_CURRENT_SOURCE_DIR}/compat/include"
        "${CMAKE_CURRENT_SOURCE_DIR}/include"
    PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/vendor/unreangel/source"
)
target_include_directories(gore-as-unreangel-core SYSTEM
    PUBLIC
        "${CMAKE_CURRENT_SOURCE_DIR}/vendor/unreangel/include"
    PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/vendor/unreangel/source"
)
target_compile_definitions(gore-as-unreangel-core PRIVATE
    ANGELSCRIPTCODE_API=
    WIN32_LEAN_AND_MEAN
    NOMINMAX
    _CRT_SECURE_NO_WARNINGS
)
# The pinned third-party core predates current MSVC warning policy. Keep its
# bytes exact and apply strict warnings to our adapters/tests instead.
target_compile_options(gore-as-unreangel-core PRIVATE
    $<$<COMPILE_LANGUAGE:CXX>:/W0>
    $<$<COMPILE_LANGUAGE:CXX>:/utf-8>
    $<$<COMPILE_LANGUAGE:CXX>:/EHsc>
    $<$<COMPILE_LANGUAGE:CXX>:/FICoreMinimal.h>
)
set_source_files_properties(
    compat/src/compat_runtime.cpp
    src/core.cpp
    src/frontend_compile.cpp
    src/json.cpp
    src/module_preprocessor.cpp
    src/precompiled_data.cpp
    src/precompiled_engine.cpp
    src/precompiled_metadata.cpp
    src/profile_loader.cpp
    src/registry_profile.cpp
    src/sha256.cpp
    src/sidecar_compile.cpp
    PROPERTIES COMPILE_OPTIONS "/W4;/WX;/permissive-;/sdl"
)
set_property(TARGET gore-as-unreangel-core PROPERTY
    MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
)

add_executable(gore-as-standalone-compiler
    src/main.cpp
)
target_link_libraries(gore-as-standalone-compiler PRIVATE gore-as-unreangel-core)

target_compile_features(gore-as-standalone-compiler PRIVATE cxx_std_17)
target_include_directories(gore-as-standalone-compiler PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
)

# Keep the sidecar independent of a separately installed MSVC runtime. Windows
# system libraries used by the standard library remain normal OS dependencies.
set_property(TARGET gore-as-standalone-compiler PROPERTY
    MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
)

target_compile_definitions(gore-as-standalone-compiler PRIVATE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
    UNICODE
    _UNICODE
)

target_compile_options(gore-as-standalone-compiler PRIVATE
    /W4
    /WX
    /permissive-
    /utf-8
    /sdl
    /guard:cf
    /Brepro
)

target_link_options(gore-as-standalone-compiler PRIVATE
    /guard:cf
    /DYNAMICBASE
    /NXCOMPAT
    /Brepro
)

include(CTest)
if(BUILD_TESTING)
    add_executable(gore-as-sidecar-compile-smoke tests/sidecar_compile_smoke.cpp)
    target_link_libraries(gore-as-sidecar-compile-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-sidecar-compile-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-sidecar-compile-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-sidecar-compile-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-protocol-primitives-smoke tests/protocol_primitives_smoke.cpp)
    target_link_libraries(gore-as-protocol-primitives-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-protocol-primitives-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-protocol-primitives-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-protocol-primitives-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-unreangel-core-smoke tests/core_smoke.cpp)
    target_link_libraries(gore-as-unreangel-core-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-unreangel-core-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-unreangel-core-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-unreangel-core-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-unreangel-graph-smoke tests/module_graph_smoke.cpp)
    target_link_libraries(gore-as-unreangel-graph-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-unreangel-graph-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-unreangel-graph-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-unreangel-graph-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-precompiled-codec-smoke tests/precompiled_codec_smoke.cpp)
    target_link_libraries(gore-as-precompiled-codec-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-precompiled-codec-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-precompiled-codec-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-precompiled-codec-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-precompiled-engine-smoke tests/precompiled_engine_smoke.cpp)
    target_link_libraries(gore-as-precompiled-engine-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-precompiled-engine-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-precompiled-engine-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-precompiled-engine-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-mixed-engine-smoke tests/mixed_engine_smoke.cpp)
    target_link_libraries(gore-as-mixed-engine-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-mixed-engine-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-mixed-engine-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-mixed-engine-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(
        gore-as-precompiled-metadata-smoke
        tests/precompiled_metadata_smoke.cpp
    )
    target_link_libraries(
        gore-as-precompiled-metadata-smoke PRIVATE gore-as-unreangel-core
    )
    target_compile_options(gore-as-precompiled-metadata-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-precompiled-metadata-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-precompiled-metadata-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-registry-profile-smoke tests/registry_profile_smoke.cpp)
    target_link_libraries(gore-as-registry-profile-smoke PRIVATE gore-as-unreangel-core)
    target_include_directories(gore-as-registry-profile-smoke SYSTEM PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/vendor/unreangel/source"
    )
    target_compile_options(gore-as-registry-profile-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-registry-profile-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-registry-profile-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-module-preprocessor-smoke tests/module_preprocessor_smoke.cpp)
    target_link_libraries(gore-as-module-preprocessor-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-module-preprocessor-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-module-preprocessor-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-module-preprocessor-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_executable(gore-as-frontend-compile-smoke tests/frontend_compile_smoke.cpp)
    target_link_libraries(gore-as-frontend-compile-smoke PRIVATE gore-as-unreangel-core)
    target_compile_options(gore-as-frontend-compile-smoke PRIVATE
        /W4 /WX /permissive- /utf-8 /sdl /guard:cf /Brepro
    )
    target_link_options(gore-as-frontend-compile-smoke PRIVATE
        /guard:cf /DYNAMICBASE /NXCOMPAT /Brepro
    )
    set_property(TARGET gore-as-frontend-compile-smoke PROPERTY
        MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>"
    )

    add_test(
        NAME gore-as-protocol.compile-smoke
        COMMAND "$<TARGET_FILE:gore-as-sidecar-compile-smoke>"
    )
    add_test(
        NAME gore-as-protocol.primitives-smoke
        COMMAND "$<TARGET_FILE:gore-as-protocol-primitives-smoke>"
    )
    add_test(
        NAME gore-as-unreangel-core.build-smoke
        COMMAND "$<TARGET_FILE:gore-as-unreangel-core-smoke>"
    )
    add_test(
        NAME gore-as-unreangel-core.module-graph-smoke
        COMMAND "$<TARGET_FILE:gore-as-unreangel-graph-smoke>"
    )
    add_test(
        NAME gore-as-precompiled-data.codec-smoke
        COMMAND "$<TARGET_FILE:gore-as-precompiled-codec-smoke>"
    )
    add_test(
        NAME gore-as-precompiled-data.engine-smoke
        COMMAND "$<TARGET_FILE:gore-as-precompiled-engine-smoke>"
    )
    add_test(
        NAME gore-as-precompiled-data.mixed-engine-smoke
        COMMAND "$<TARGET_FILE:gore-as-mixed-engine-smoke>"
    )
    add_test(
        NAME gore-as-precompiled-data.metadata-smoke
        COMMAND "$<TARGET_FILE:gore-as-precompiled-metadata-smoke>"
    )
    add_test(
        NAME gore-as-registry-profile.replay-smoke
        COMMAND "$<TARGET_FILE:gore-as-registry-profile-smoke>"
    )
    add_test(
        NAME gore-as-module-preprocessor.lexical-smoke
        COMMAND "$<TARGET_FILE:gore-as-module-preprocessor-smoke>"
    )
    add_test(
        NAME gore-as-frontend-compile.graph-smoke
        COMMAND "$<TARGET_FILE:gore-as-frontend-compile-smoke>"
    )
    add_test(
        NAME gore-as-standalone-compiler.cli-smoke
        COMMAND "${CMAKE_COMMAND}"
            "-DEXECUTABLE=$<TARGET_FILE:gore-as-standalone-compiler>"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/tests/cli_smoke.cmake"
    )
endif()

install(TARGETS gore-as-standalone-compiler RUNTIME DESTINATION bin)
