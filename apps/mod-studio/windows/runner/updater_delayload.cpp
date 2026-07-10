// Delay-load shim for the auto-updater plugin.
//
// generated_plugin_registrant.cc links the runner exe against
// auto_updater_windows_plugin.dll at load time, importing the single symbol
// AutoUpdaterWindowsPluginCApiRegisterWithRegistrar. The portable build ships
// without that DLL (NexusMods quarantines it as a false-positive virus), so a
// plain load-time import would make the exe fail to start.
//
// We delay-load the plugin DLL (see runner CMakeLists /DELAYLOAD) and install a
// failure hook: when the DLL is absent, registration resolves to a no-op stub
// and the app starts updater-free. The Dart side never calls the updater in a
// portable build (it is gated to Inno-installed copies), so the missing handler
// is never exercised. Installer builds bundle the DLL, the hook never fires, and
// the real plugin registers normally.

#include <windows.h>

#include <delayimp.h>
#include <string.h>

namespace {

// Matches the C ABI of AutoUpdaterWindowsPluginCApiRegisterWithRegistrar
// (FlutterDesktopPluginRegistrarRef is an opaque pointer). Does nothing: the
// plugin simply stays unregistered.
void NoopRegisterWithRegistrar(void* /*registrar*/) {}

bool IsUpdaterPlugin(const char* dll) {
  return dll && _stricmp(dll, "auto_updater_windows_plugin.dll") == 0;
}

FARPROC WINAPI DelayLoadFailureHook(unsigned notify, PDelayLoadInfo info) {
  if (info == nullptr || !IsUpdaterPlugin(info->szDll)) {
    return nullptr;  // not ours: let the default (crash) behaviour stand.
  }
  // LoadLibrary failed: hand back any valid module so the helper proceeds to
  // proc resolution, which the dliFailGetProc branch then satisfies.
  if (notify == dliFailLoadLib) {
    return reinterpret_cast<FARPROC>(GetModuleHandleW(nullptr));
  }
  // Proc lookup failed (the exe doesn't export it): supply the no-op stub.
  if (notify == dliFailGetProc) {
    return reinterpret_cast<FARPROC>(&NoopRegisterWithRegistrar);
  }
  return nullptr;
}

}  // namespace

// The delayimp helper consults this global on load/proc failure.
extern "C" const PfnDliHook __pfnDliFailureHook2 = DelayLoadFailureHook;
