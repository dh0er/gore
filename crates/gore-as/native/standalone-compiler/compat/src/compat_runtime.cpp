// Standalone definitions for the tiny set of plugin globals referenced by the
// pinned core. Profile-driven values will replace defaults that affect compile
// policy; this runtime-only flag is false for a generic non-game smoke build.
#include "angelscript.h"

#include <clocale>
#include <cstdlib>

thread_local bool GIsAngelscriptWorldContextAvailable = false;
thread_local bool GIsInAngelscriptThreadSafeFunction = false;

// UNREANGEL resolves this through UASClass/UObject metadata in
// Private/angelscript.cpp. A compiler-only sidecar never constructs or invokes
// a UObject-backed asIScriptObject. Reaching this method would therefore mean
// that a captured "compile_only_never_invoke" host boundary was violated.
// Abort instead of returning a fabricated/null type and silently changing
// compile-time callback behaviour; the process adapter reports the failed
// standalone attempt and may use the explicitly requested game fallback.
asITypeInfo* asIScriptObject::GetObjectType() const {
    std::abort();
}

// FCStringAnsi numeric parsing uses the C numeric grammar. Pin it explicitly so
// a user's process locale cannot reinterpret AngelScript's mandatory '.'
// decimal separator. Atof follows Unreal's double-parse-then-float conversion,
// which can differ from a direct strtof rounding at boundary values.
double asStringScanDouble(const char* text) {
#if defined(_WIN32)
    static _locale_t c_locale = _create_locale(LC_NUMERIC, "C");
    if (c_locale == nullptr) std::abort();
    return _strtod_l(text, nullptr, c_locale);
#else
    return std::strtod(text, nullptr);
#endif
}

float asStringScanFloat(const char* text) {
    return static_cast<float>(asStringScanDouble(text));
}
