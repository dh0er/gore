// Standalone definitions for the tiny set of plugin globals referenced by the
// pinned core. Profile-driven values will replace defaults that affect compile
// policy; this runtime-only flag is false for a generic non-game smoke build.
#include "angelscript.h"

#include <cstdlib>

thread_local bool GIsAngelscriptWorldContextAvailable = false;

// UNREANGEL resolves this through UASClass/UObject metadata in
// Private/angelscript.cpp. Standalone object metadata is profile work; return
// no type instead of manufacturing an incompatible UObject layout.
asITypeInfo* asIScriptObject::GetObjectType() const {
    return nullptr;
}

// UNREANGEL routes these through FCStringAnsi in AngelscriptManager.cpp. The
// CRT routines provide the same locale-sensitive numeric scan without pulling
// the Unreal manager into the compiler process.
double asStringScanDouble(const char* text) {
    return std::strtod(text, nullptr);
}

float asStringScanFloat(const char* text) {
    return std::strtof(text, nullptr);
}
