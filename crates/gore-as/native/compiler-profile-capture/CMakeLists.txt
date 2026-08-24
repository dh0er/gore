cmake_minimum_required(VERSION 3.25)
project(gore_as_compiler_profile_capture LANGUAGES CXX ASM_MASM)

include(CTest)
option(GORE_AS_CAPTURE_BUILD_LIVE_CONTROLLER
  "Build the explicitly authorized exact-target windowed capture launcher" OFF)

# VS' MASM integration otherwise mirrors the source subdirectory into ObjectFileName without
# creating it. $(IntDir) is already target/configuration-specific for both bridge variants.
set_source_files_properties(src/production_observer_shims.asm PROPERTIES
  VS_SETTINGS "ObjectFileName=$(IntDir)production_observer_shims.obj")

if(NOT WIN32 OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "compiler-profile-capture supports only Windows x64")
endif()

function(gore_as_capture_configure_target target)
  target_compile_features(${target} PRIVATE cxx_std_20)
  target_compile_definitions(${target} PRIVATE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
    _WIN32_WINNT=0x0A00
  )
  if(MSVC)
    target_compile_options(${target} PRIVATE
      $<$<COMPILE_LANGUAGE:CXX>:/W4 /WX /permissive- /utf-8 /guard:cf /Zc:preprocessor>)
    set_property(TARGET ${target} PROPERTY
      MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
    get_target_property(target_type ${target} TYPE)
    if(NOT target_type STREQUAL "STATIC_LIBRARY")
      target_link_options(${target} PRIVATE /guard:cf /DYNAMICBASE /NXCOMPAT)
    endif()
  endif()
endfunction()

add_library(gore_as_compiler_profile_capture STATIC
  src/path_safety.cpp
  src/session.cpp
)
target_include_directories(gore_as_compiler_profile_capture PUBLIC include)
target_link_libraries(gore_as_compiler_profile_capture PRIVATE bcrypt)
gore_as_capture_configure_target(gore_as_compiler_profile_capture)

add_library(gore_as_compiler_profile_capture_bridge SHARED
  src/bridge.cpp
  src/instrumentation.cpp
  src/live_bootstrap.cpp
  src/production_capture_phase_machine.cpp
  src/production_capture_dispatcher.cpp
  src/production_observer_shims.asm
  src/production_observer_shims.cpp
  src/target_capture_serializer.cpp
  src/target_final_state.cpp
  src/target_frontend_observer.cpp
  src/target_frontend_raw_materializer.cpp
  src/target_frontend_snapshot_builder.cpp
  src/target_registration_observer.cpp
  src/target_snapshot.cpp
  src/target_type_usage.cpp
)
target_include_directories(gore_as_compiler_profile_capture_bridge PUBLIC include)
target_link_libraries(gore_as_compiler_profile_capture_bridge PRIVATE
  gore_as_compiler_profile_capture
  bcrypt
)
gore_as_capture_configure_target(gore_as_compiler_profile_capture_bridge)

add_library(gore_as_capture_materializer STATIC src/materializer.cpp)
target_include_directories(gore_as_capture_materializer PUBLIC include PRIVATE src)
target_link_libraries(gore_as_capture_materializer PRIVATE bcrypt)
gore_as_capture_configure_target(gore_as_capture_materializer)

add_executable(gore_as_capture_materializer_tool tools/materializer_main.cpp)
target_include_directories(gore_as_capture_materializer_tool PRIVATE include src)
target_link_libraries(gore_as_capture_materializer_tool PRIVATE gore_as_capture_materializer)
set_target_properties(gore_as_capture_materializer_tool PROPERTIES
  OUTPUT_NAME gore_as_capture_materializer)
gore_as_capture_configure_target(gore_as_capture_materializer_tool)

add_executable(gore_as_capture_offline_host tools/offline_host.cpp)
target_include_directories(gore_as_capture_offline_host PRIVATE include src)
target_link_libraries(gore_as_capture_offline_host PRIVATE gore_as_capture_materializer)
gore_as_capture_configure_target(gore_as_capture_offline_host)

if(GORE_AS_CAPTURE_BUILD_LIVE_CONTROLLER)
  add_executable(gore_as_capture_live_controller tools/live_capture_controller.cpp)
  target_include_directories(gore_as_capture_live_controller PRIVATE include)
  target_link_libraries(gore_as_capture_live_controller PRIVATE bcrypt)
  gore_as_capture_configure_target(gore_as_capture_live_controller)
endif()

if(BUILD_TESTING)
  # This target is intentionally a separate byte image and never an install/export artifact. It
  # relaxes only the executable byte/CodeView check so the host executable can act as a fake
  # primary image. It retains source-handle identity, CREATE_NEW output, wire ordering and seals.
  add_library(gore_as_capture_fixture_writer STATIC
    src/path_safety.cpp
    src/session.cpp
  )
  target_include_directories(gore_as_capture_fixture_writer PUBLIC include)
  target_compile_definitions(gore_as_capture_fixture_writer PRIVATE
    GORE_AS_CAPTURE_TEST_TARGET=1)
  target_link_libraries(gore_as_capture_fixture_writer PRIVATE bcrypt)
  gore_as_capture_configure_target(gore_as_capture_fixture_writer)

  add_library(gore_as_capture_bridge_fixture SHARED
    src/bridge.cpp
    src/instrumentation.cpp
    src/live_bootstrap.cpp
    src/production_capture_phase_machine.cpp
    src/production_capture_dispatcher.cpp
    src/production_observer_shims.asm
    src/production_observer_shims.cpp
    src/target_capture_serializer.cpp
    src/target_final_state.cpp
    src/target_frontend_observer.cpp
    src/target_frontend_raw_materializer.cpp
    src/target_frontend_snapshot_builder.cpp
    src/target_registration_observer.cpp
    src/target_snapshot.cpp
    src/target_type_usage.cpp
  )
  target_include_directories(gore_as_capture_bridge_fixture PRIVATE include)
  target_compile_definitions(gore_as_capture_bridge_fixture PRIVATE
    GORE_AS_CAPTURE_TEST_TARGET=1)
  target_link_libraries(gore_as_capture_bridge_fixture PRIVATE
    gore_as_capture_fixture_writer
    bcrypt
  )
  gore_as_capture_configure_target(gore_as_capture_bridge_fixture)

  add_executable(gore_as_capture_path_safety_test tests/path_safety_test.cpp)
  target_include_directories(gore_as_capture_path_safety_test PRIVATE src)
  target_link_libraries(gore_as_capture_path_safety_test PRIVATE
    gore_as_compiler_profile_capture
  )
  gore_as_capture_configure_target(gore_as_capture_path_safety_test)
  add_test(NAME gore_as_capture_path_safety COMMAND gore_as_capture_path_safety_test)

  # Compile the pinned standalone donor headers under the target's Shipping defines. This is an
  # offline layout witness only; no donor code is linked into the capture DLL.
  set(gore_as_capture_donor_root
    "${CMAKE_CURRENT_SOURCE_DIR}/../standalone-compiler")
  add_executable(gore_as_capture_donor_layout_test tests/donor_layout_test.cpp)
  target_include_directories(gore_as_capture_donor_layout_test PRIVATE
    src
    "${gore_as_capture_donor_root}/compat/include")
  target_include_directories(gore_as_capture_donor_layout_test SYSTEM PRIVATE
    "${gore_as_capture_donor_root}/vendor/unreangel/include"
    "${gore_as_capture_donor_root}/vendor/unreangel/source")
  target_compile_definitions(gore_as_capture_donor_layout_test PRIVATE
    ANGELSCRIPTCODE_API=
    WITH_EDITOR=0
    AS_REFERENCE_DEBUGGING=0)
  target_compile_options(gore_as_capture_donor_layout_test PRIVATE /FICoreMinimal.h)
  gore_as_capture_configure_target(gore_as_capture_donor_layout_test)
  add_test(NAME gore_as_capture_donor_layout COMMAND gore_as_capture_donor_layout_test)

  add_test(
    NAME gore_as_capture_synthetic_e2e
    COMMAND gore_as_capture_offline_host
      --synthetic-e2e
      $<TARGET_FILE:gore_as_capture_bridge_fixture>
      $<TARGET_FILE:gore_as_compiler_profile_capture_bridge>
  )

  if(MSVC)
    get_filename_component(gore_as_capture_linker_directory "${CMAKE_LINKER}" DIRECTORY)
    add_test(
      NAME gore_as_capture_production_pe_imports
      COMMAND "${CMAKE_COMMAND}"
        "-DDUMPBIN=${gore_as_capture_linker_directory}/dumpbin.exe"
        "-DPE=$<TARGET_FILE:gore_as_compiler_profile_capture_bridge>"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/tests/pe_import_audit.cmake"
    )
  endif()
endif()
