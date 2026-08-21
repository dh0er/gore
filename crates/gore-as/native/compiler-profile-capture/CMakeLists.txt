cmake_minimum_required(VERSION 3.25)
project(gore_as_compiler_profile_capture LANGUAGES CXX)

include(CTest)

if(NOT WIN32 OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "compiler-profile-capture supports only Windows x64")
endif()
add_library(gore_as_compiler_profile_capture STATIC
  src/path_safety.cpp
  src/session.cpp
)
target_compile_features(gore_as_compiler_profile_capture PUBLIC cxx_std_20)
target_include_directories(gore_as_compiler_profile_capture PUBLIC include)
target_compile_definitions(gore_as_compiler_profile_capture PRIVATE
  WIN32_LEAN_AND_MEAN
  NOMINMAX
  _WIN32_WINNT=0x0A00
)
target_link_libraries(gore_as_compiler_profile_capture PRIVATE bcrypt)

if(MSVC)
  target_compile_options(gore_as_compiler_profile_capture PRIVATE /W4 /WX /permissive-)
endif()

if(BUILD_TESTING)
  add_executable(gore_as_capture_path_safety_test tests/path_safety_test.cpp)
  target_compile_features(gore_as_capture_path_safety_test PRIVATE cxx_std_20)
  target_include_directories(gore_as_capture_path_safety_test PRIVATE src)
  target_compile_definitions(gore_as_capture_path_safety_test PRIVATE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
    _WIN32_WINNT=0x0A00
  )
  target_link_libraries(gore_as_capture_path_safety_test PRIVATE
    gore_as_compiler_profile_capture
  )
  if(MSVC)
    target_compile_options(gore_as_capture_path_safety_test PRIVATE /W4 /WX /permissive-)
  endif()
  add_test(NAME gore_as_capture_path_safety COMMAND gore_as_capture_path_safety_test)
endif()
