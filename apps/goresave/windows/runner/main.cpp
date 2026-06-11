#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include <string>

#include "flutter_window.h"
#include "utils.h"

namespace {

// Runs Velopack install/update hooks (--veloapp-install etc.) from the core
// DLL next to the executable. Velopack may exit the process here, so this
// must run before any Flutter or COM initialization. A missing DLL or export
// is fine (dev run without a built core).
void RunVelopackStartupHooks() {
  wchar_t exe_path[MAX_PATH];
  if (::GetModuleFileNameW(nullptr, exe_path, MAX_PATH) == 0) {
    return;
  }
  std::wstring dll_path(exe_path);
  size_t last_slash = dll_path.find_last_of(L"\\/");
  if (last_slash == std::wstring::npos) {
    return;
  }
  dll_path = dll_path.substr(0, last_slash + 1) + L"goresave_core.dll";
  // Full path, so the Windows DLL search order cannot substitute a
  // same-named DLL from elsewhere on the search path.
  HMODULE core = ::LoadLibraryExW(dll_path.c_str(), nullptr,
                                  LOAD_WITH_ALTERED_SEARCH_PATH);
  if (core == nullptr) {
    return;
  }
  using StartupFn = void (*)();
  auto startup = reinterpret_cast<StartupFn>(
      ::GetProcAddress(core, "goresave_velopack_startup"));
  if (startup != nullptr) {
    startup();
  }
}

}  // namespace

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  // Must run before anything else: Velopack hooks may exit the process.
  RunVelopackStartupHooks();

  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  flutter::DartProject project(L"data");

  std::vector<std::string> command_line_arguments =
      GetCommandLineArguments();

  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project);
  Win32Window::Point origin(10, 10);
  Win32Window::Size size(1600, 900);
  if (!window.Create(L"Gothic Remake Savegame Editor", origin, size)) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }

  ::CoUninitialize();
  return EXIT_SUCCESS;
}
