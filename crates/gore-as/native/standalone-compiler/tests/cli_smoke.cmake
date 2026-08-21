if(NOT DEFINED EXECUTABLE OR NOT EXISTS "${EXECUTABLE}")
    message(FATAL_ERROR "EXECUTABLE must name the built sidecar")
endif()

execute_process(
    COMMAND "${EXECUTABLE}" --version
    RESULT_VARIABLE version_result
    OUTPUT_VARIABLE version_stdout
    ERROR_VARIABLE version_stderr
)
if(NOT version_result EQUAL 0 OR
   NOT version_stdout MATCHES "^gore-as-standalone-compiler 0\\.1\\.0-dev")
    message(FATAL_ERROR "--version failed: ${version_result}; ${version_stdout}; ${version_stderr}")
endif()

execute_process(
    COMMAND "${EXECUTABLE}" --capabilities
    RESULT_VARIABLE capabilities_result
    OUTPUT_VARIABLE capabilities_stdout
    ERROR_VARIABLE capabilities_stderr
)
if(NOT capabilities_result EQUAL 0 OR
   NOT capabilities_stdout MATCHES "\\\"request_version\\\":1" OR
   NOT capabilities_stdout MATCHES "\\\"response_version\\\":1" OR
   NOT capabilities_stdout MATCHES "\\\"core\\\":\\{\\\"available\\\":true" OR
   NOT capabilities_stdout MATCHES "247954da5326ecc29724067da7b5880c352fe4ff" OR
   NOT capabilities_stdout MATCHES "\\\"available\\\":false" OR
   NOT capabilities_stdout MATCHES "\\\"requires_unreal_runtime\\\":false" OR
   NOT capabilities_stdout MATCHES "\\\"requires_game_dll\\\":false")
    message(FATAL_ERROR "--capabilities failed: ${capabilities_result}; ${capabilities_stdout}; ${capabilities_stderr}")
endif()

set(request_file "${CMAKE_CURRENT_BINARY_DIR}/standalone-compiler-smoke-request.json")
file(WRITE "${request_file}" "{\"request_version\":1,\"operation\":\"compile\"}\n")
execute_process(
    COMMAND "${EXECUTABLE}" compile --request "${request_file}"
    RESULT_VARIABLE compile_result
    OUTPUT_VARIABLE compile_stdout
    ERROR_VARIABLE compile_stderr
)
file(REMOVE "${request_file}")
if(NOT compile_result EQUAL 69 OR
   NOT compile_stdout MATCHES "\\\"response_version\\\":1" OR
   NOT compile_stdout MATCHES "\\\"ok\\\":false" OR
   NOT compile_stdout MATCHES "GORE_AS_STANDALONE_ENGINE_UNAVAILABLE")
    message(FATAL_ERROR "compile stub did not fail closed: ${compile_result}; ${compile_stdout}; ${compile_stderr}")
endif()

execute_process(
    COMMAND "${EXECUTABLE}" compile --request "${request_file}"
    RESULT_VARIABLE missing_result
    OUTPUT_VARIABLE missing_stdout
    ERROR_VARIABLE missing_stderr
)
if(NOT missing_result EQUAL 65 OR
   NOT missing_stdout MATCHES "\\\"ok\\\":false" OR
   NOT missing_stdout MATCHES "GORE_AS_REQUEST_OPEN_FAILED")
    message(FATAL_ERROR "missing request was not rejected: ${missing_result}; ${missing_stdout}; ${missing_stderr}")
endif()

set(oversized_file "${CMAKE_CURRENT_BINARY_DIR}/standalone-compiler-oversized-request.json")
string(REPEAT "x" 1048577 oversized_request)
file(WRITE "${oversized_file}" "${oversized_request}")
unset(oversized_request)
execute_process(
    COMMAND "${EXECUTABLE}" compile --request "${oversized_file}"
    RESULT_VARIABLE oversized_result
    OUTPUT_VARIABLE oversized_stdout
    ERROR_VARIABLE oversized_stderr
)
file(REMOVE "${oversized_file}")
if(NOT oversized_result EQUAL 65 OR
   NOT oversized_stdout MATCHES "GORE_AS_REQUEST_TOO_LARGE")
    message(FATAL_ERROR "oversized request was not rejected: ${oversized_result}; ${oversized_stdout}; ${oversized_stderr}")
endif()

set(nested_file "${CMAKE_CURRENT_BINARY_DIR}/standalone-compiler-nested-request.json")
set(nested_request "{\"value\":")
foreach(_index RANGE 1 33)
    string(APPEND nested_request "[")
endforeach()
string(APPEND nested_request "0")
foreach(_index RANGE 1 33)
    string(APPEND nested_request "]")
endforeach()
string(APPEND nested_request "}")
file(WRITE "${nested_file}" "${nested_request}")
execute_process(
    COMMAND "${EXECUTABLE}" compile --request "${nested_file}"
    RESULT_VARIABLE nested_result
    OUTPUT_VARIABLE nested_stdout
    ERROR_VARIABLE nested_stderr
)
file(REMOVE "${nested_file}")
if(NOT nested_result EQUAL 65 OR
   NOT nested_stdout MATCHES "GORE_AS_REQUEST_NESTING_INVALID")
    message(FATAL_ERROR "deep request was not rejected: ${nested_result}; ${nested_stdout}; ${nested_stderr}")
endif()
