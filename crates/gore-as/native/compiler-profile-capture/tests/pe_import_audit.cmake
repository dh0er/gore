if(NOT DEFINED DUMPBIN OR NOT EXISTS "${DUMPBIN}")
  message(FATAL_ERROR "dumpbin.exe is required for the capture bridge import audit")
endif()
if(NOT DEFINED PE OR NOT EXISTS "${PE}")
  message(FATAL_ERROR "production capture bridge is missing")
endif()

execute_process(
  COMMAND "${DUMPBIN}" /nologo /imports "${PE}"
  RESULT_VARIABLE dumpbin_result
  OUTPUT_VARIABLE imports
  ERROR_VARIABLE dumpbin_error
)
if(NOT dumpbin_result EQUAL 0)
  message(FATAL_ERROR "dumpbin /imports failed: ${dumpbin_error}")
endif()

string(TOLOWER "${imports}" imports_lower)
foreach(forbidden IN ITEMS
    openprocess
    createthread
    createremotethread
    ntcreatethreadex
    writeprocessmemory
    readprocessmemory
    virtualallocex
    virtualprotectex
    queueuserapc
    setwindowshookex
    debugactiveprocess
    createprocess
    shellexecute
    winexec)
  string(FIND "${imports_lower}" "${forbidden}" forbidden_offset)
  if(NOT forbidden_offset EQUAL -1)
    message(FATAL_ERROR "forbidden remote-process/injection/launch import: ${forbidden}")
  endif()
endforeach()

string(REGEX MATCHALL "[\r\n][ \t]+[A-Za-z0-9_.-]+\\.[dD][lL][lL]" imported_dll_lines "${imports}")
set(imported_dlls)
foreach(imported_dll_line IN LISTS imported_dll_lines)
  string(STRIP "${imported_dll_line}" imported_dll)
  string(TOLOWER "${imported_dll}" imported_dll)
  list(APPEND imported_dlls "${imported_dll}")
endforeach()
list(REMOVE_DUPLICATES imported_dlls)
list(SORT imported_dlls)

set(expected_dlls bcrypt.dll kernel32.dll)
if(NOT imported_dlls STREQUAL expected_dlls)
  message(FATAL_ERROR
    "production capture bridge dependency drift: expected '${expected_dlls}', got '${imported_dlls}'")
endif()
