// Temporary G1R AngelScript and ClassGenerator diagnostic capture helper.
//
// Hooks the UE-AngelScript per-message callback and FAngelscriptManager's structured diagnostic
// insertion boundary. The Rust launcher independently scans the same masked AOBs and sparse body
// fingerprints offline and injects the DLL only when all are valid. The DLL repeats those checks
// in the mapped .text section and reports `ready` only after MinHook enables both hooks. No game
// file is patched and no DLL is installed permanently.

#include <windows.h>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include "MinHook.h"

// LogAngelscriptError(asSMessageInfo*, void*). Observed at RVA 0x4685ff0 in BuildID 24539464
// (1.0.2: 0x467e200); RVAs are documentation only and never used for lookup.
static const char* kLogErrSig =
    "40 55 56 57 48 8D AC 24 60 FF FF FF 48 81 EC A0 01 00 00 48 8B 05 ?? ?? ?? ?? "
    "48 33 C4 48 89 85 80 00 00 00 8B 15 ?? ??";

// Sparse callback-body fingerprint, kept byte-for-byte equivalent to diagnostics.rs. It proves
// the first argument and all five asSMessageInfo field offsets without hashing relocations, call
// targets, branch displacements or local stack destinations.
struct ShapeClause {
    size_t offset;
    const char* signature;
};

static const size_t kCallbackShapeSpan = 0x244;
static const ShapeClause kCallbackShape[] = {
    {0x02a, "48 8B F9"},
    {0x09a, "48 8B 17"},
    {0x119, "44 39 6F 0C 75 ?? 44 39 6F 08 75 ?? 48 8B 57 18"},
    {0x233, "8B 47 08 89 44 24 ?? 8B 47 0C 89 44 24 ?? 8B 47 10"},
};

// FAngelscriptManager::ScriptCompileError(const FString&, const FDiagnostic&). Observed at RVA
// 0x4689e80 in BuildID 24539464; the same AOB is unique in every archived 1.0.0..1.0.4a EXE.
// RVAs are documentation only and never used for lookup.
static const char* kManagerDiagnosticSig =
    "48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 C6 81 58 04 00 00 01 "
    "48 8D 99 B8 03 00 00 8B 42 08";
static const size_t kManagerDiagnosticShapeSpan = 0xb0;
static const ShapeClause kManagerDiagnosticShape[] = {
    {0x01d, "8B 42 08 49 8B F8 48 8B F2"},
    {0x02a, "48 8B 12"},
    {0x083, "48 8B D7"},
    {0x091, "8B 47 10 48 8B 74 24 ?? 89 43 10 8B 47 14 89 43 14"},
    {0x0a2, "0F B6 47 18 88 43 18 0F B6 47 19 88 43 19"},
};

struct asMsgInfo {
    const char* section;
    int32_t row;
    int32_t col;
    int32_t type;
    const char* message;
};

typedef void (*LogErr_t)(void*, void*);
static LogErr_t g_orig_logerr = nullptr;

struct UnrealString {
    const wchar_t* data;
    int32_t num;
    int32_t max;
};

struct ManagerDiagnostic {
    UnrealString message;
    int32_t row;
    int32_t col;
    uint8_t is_error;
    uint8_t is_info;
};

typedef void (*ManagerDiagnostic_t)(void*, const UnrealString*, const ManagerDiagnostic*);
static ManagerDiagnostic_t g_orig_manager_diagnostic = nullptr;
static CRITICAL_SECTION g_cs;
static char g_last_section[2048] = {0};
static HANDLE g_errh = INVALID_HANDLE_VALUE;
static uint64_t g_capture_bytes = 0;
static const uint64_t MAX_CAPTURE_BYTES = 8ull * 1024ull * 1024ull;
static const char kCaptureTruncated[] =
    "\n[GORE] diagnostics capture truncated at 8 MiB\n";
static bool g_capture_truncated = false;
static const size_t MAX_TEXT_BYTES = 1024ull * 1024ull * 1024ull;

static void report_status(const char* value) {
    wchar_t pathw[MAX_PATH];
    DWORD n = GetEnvironmentVariableW(L"GORE_AS_STATUSFILE", pathw, MAX_PATH);
    if (!n || n >= MAX_PATH) return;
    HANDLE h = CreateFileW(pathw, GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
                           nullptr, CREATE_ALWAYS, FILE_FLAG_WRITE_THROUGH, nullptr);
    if (h == INVALID_HANDLE_VALUE) return;
    char line[256];
    int length = snprintf(line, sizeof line, "%s\n", value);
    if (length < 0) {
        CloseHandle(h);
        return;
    }
    if (length >= static_cast<int>(sizeof line)) length = sizeof line - 1;
    DWORD written = 0;
    WriteFile(h, line, static_cast<DWORD>(length), &written, nullptr);
    FlushFileBuffers(h);
    CloseHandle(h);
}

static bool readable(const void* ptr, size_t bytes) {
    if (!ptr || bytes == 0) return false;
    MEMORY_BASIC_INFORMATION mbi;
    if (!VirtualQuery(ptr, &mbi, sizeof mbi) || mbi.State != MEM_COMMIT) return false;
    const DWORD readable_flags = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ |
        PAGE_EXECUTE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_WRITECOPY;
    if (!(mbi.Protect & readable_flags) || (mbi.Protect & PAGE_GUARD)) return false;
    uintptr_t start = reinterpret_cast<uintptr_t>(ptr);
    uintptr_t region_end = reinterpret_cast<uintptr_t>(mbi.BaseAddress) + mbi.RegionSize;
    return start <= region_end && bytes <= region_end - start;
}

static bool readable_span(const void* ptr, size_t bytes) {
    if (!ptr || !bytes) return false;
    uintptr_t cursor = reinterpret_cast<uintptr_t>(ptr);
    if (bytes > UINTPTR_MAX - cursor) return false;
    const uintptr_t end = cursor + bytes;
    while (cursor < end) {
        MEMORY_BASIC_INFORMATION mbi;
        if (!VirtualQuery(reinterpret_cast<const void*>(cursor), &mbi, sizeof mbi) ||
            mbi.State != MEM_COMMIT)
            return false;
        const DWORD readable_flags = PAGE_READONLY | PAGE_READWRITE | PAGE_EXECUTE_READ |
            PAGE_EXECUTE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_WRITECOPY;
        if (!(mbi.Protect & readable_flags) || (mbi.Protect & PAGE_GUARD)) return false;
        const uintptr_t region_end = reinterpret_cast<uintptr_t>(mbi.BaseAddress) + mbi.RegionSize;
        if (region_end <= cursor) return false;
        cursor = region_end < end ? region_end : end;
    }
    return true;
}

// Bounded, page-safe copy of the AngelScript callback's ANSI pointers.
static int copy_cstr(const char* src, char* out, int capacity) {
    if (!src || capacity <= 0 || !readable(src, 1)) {
        if (capacity > 0) out[0] = 0;
        return 0;
    }
    int i = 0;
    for (; i < capacity - 1; ++i) {
        if ((reinterpret_cast<uintptr_t>(src + i) & 0xfff) == 0 && !readable(src + i, 1)) break;
        char c = src[i];
        if (!c) break;
        out[i] = c;
    }
    out[i] = 0;
    return i;
}

// Returns -1 for an invalid/oversized FString. The caller marks the entire capture incomplete
// instead of accepting a truncated diagnostic as authoritative.
static int copy_fstring_utf8(const UnrealString* src, char* out, int capacity) {
    if (!src || !out || capacity <= 0 || !readable(src, sizeof *src)) return -1;
    if (src->num < 0 || src->max < src->num || src->num > 1024 * 1024) return -1;
    if (src->num == 0) {
        out[0] = 0;
        return 0;
    }
    const size_t bytes = static_cast<size_t>(src->num) * sizeof(wchar_t);
    if (!readable_span(src->data, bytes)) return -1;
    int chars = src->num;
    if (chars > 0 && src->data[chars - 1] == L'\0') --chars;
    if (chars == 0) {
        out[0] = 0;
        return 0;
    }
    int required = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, src->data, chars,
                                       nullptr, 0, nullptr, nullptr);
    if (required <= 0 || required >= capacity) return -1;
    int written = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, src->data, chars,
                                      out, required, nullptr, nullptr);
    if (written != required) return -1;
    out[written] = 0;
    return written;
}

static void ensure_capture_open() {
    if (g_errh != INVALID_HANDLE_VALUE) return;
    wchar_t pathw[MAX_PATH];
    DWORD n = GetEnvironmentVariableW(L"GORE_AS_ERRFILE", pathw, MAX_PATH);
    if (n > 0 && n < MAX_PATH) {
        g_errh = CreateFileW(pathw, FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE,
                            nullptr, OPEN_ALWAYS, FILE_FLAG_WRITE_THROUGH, nullptr);
    }
}

static void capture_write(const char* bytes, int length) {
    if (g_errh == INVALID_HANDLE_VALUE || length <= 0 || g_capture_truncated)
        return;
    const uint64_t marker_bytes = sizeof kCaptureTruncated - 1;
    const uint64_t content_cap = MAX_CAPTURE_BYTES - marker_bytes;
    const uint64_t room = g_capture_bytes < content_cap ? content_cap - g_capture_bytes : 0;
    const DWORD want = static_cast<DWORD>(length);
    if (static_cast<uint64_t>(length) > room) {
        DWORD marker_written = 0;
        if (WriteFile(g_errh, kCaptureTruncated, static_cast<DWORD>(marker_bytes),
                      &marker_written, nullptr))
            g_capture_bytes += marker_written;
        g_capture_truncated = true;
        return;
    }
    DWORD written = 0;
    if (want && WriteFile(g_errh, bytes, want, &written, nullptr)) g_capture_bytes += written;
    if (written != want) {
        DWORD marker_written = 0;
        if (WriteFile(g_errh, kCaptureTruncated, static_cast<DWORD>(marker_bytes),
                      &marker_written, nullptr))
            g_capture_bytes += marker_written;
        g_capture_truncated = true;
    }
}

static void invalidate_capture() {
    ensure_capture_open();
    if (g_errh == INVALID_HANDLE_VALUE || g_capture_truncated) return;
    const DWORD marker_bytes = sizeof kCaptureTruncated - 1;
    DWORD written = 0;
    if (g_capture_bytes <= MAX_CAPTURE_BYTES - marker_bytes)
        WriteFile(g_errh, kCaptureTruncated, marker_bytes, &written, nullptr);
    g_capture_bytes += written;
    g_capture_truncated = true;
}

static void capture_diagnostic(const char* section, int32_t row, int32_t col, char kind,
                               const char* message) {
    char line[10240];
    EnterCriticalSection(&g_cs);
    ensure_capture_open();
    if (g_errh != INVALID_HANDLE_VALUE) {
        int length;
        if (strcmp(section, g_last_section) != 0) {
            length = snprintf(line, sizeof line, "=== %s ===\n", section[0] ? section : "(?)");
            if (length >= static_cast<int>(sizeof line)) length = sizeof line - 1;
            capture_write(line, length);
            strncpy(g_last_section, section, sizeof g_last_section - 1);
            g_last_section[sizeof g_last_section - 1] = 0;
        }
        if (row || col)
            length = snprintf(line, sizeof line, "(%d:%d) [%c] %s\n", row, col, kind, message);
        else
            length = snprintf(line, sizeof line, "[%c] %s\n", kind, message);
        if (length >= static_cast<int>(sizeof line)) length = sizeof line - 1;
        capture_write(line, length);
    }
    LeaveCriticalSection(&g_cs);
}

static void detour_logerr(void* msgp, void* data) {
    if (readable(msgp, sizeof(asMsgInfo))) {
        asMsgInfo* msg = static_cast<asMsgInfo*>(msgp);
        char section[2048], text[8192];
        copy_cstr(msg->section, section, sizeof section);
        copy_cstr(msg->message, text, sizeof text);
        // Per-function progress dominates the 7,305-module build and can consume the entire
        // bounded capture before a real warning/error arrives. It is routine information, not a
        // diagnostic; filter it before opening/writing the FILE_FLAG_WRITE_THROUGH stream.
        const bool routine_compile = msg->type == 2 && !strncmp(text, "Compiling ", 10);
        if (text[0] && !routine_compile) {
            // AngelScript asSMessageInfo: 0=error, 1=warning, 2=information.
            char kind = msg->type == 0 ? 'E' : msg->type == 1 ? 'W' : msg->type == 2 ? 'I' : '?';
            capture_diagnostic(section, msg->row, msg->col, kind, text);
        }
    }
    g_orig_logerr(msgp, data);
}

static void detour_manager_diagnostic(void* manager, const UnrealString* filename,
                                      const ManagerDiagnostic* diagnostic) {
    if (readable(filename, sizeof *filename) && readable(diagnostic, sizeof *diagnostic)) {
        char section[2048], message[8192];
        int section_length = copy_fstring_utf8(filename, section, sizeof section);
        int message_length = copy_fstring_utf8(&diagnostic->message, message, sizeof message);
        if (section_length < 0 || message_length < 0) {
            EnterCriticalSection(&g_cs);
            invalidate_capture();
            LeaveCriticalSection(&g_cs);
        } else if (section_length > 0 && message_length > 0) {
            // The game also routes post-generation runtime validation failures through this
            // manager method with an empty filename and no source position (for example missing
            // AssessmentConditionFunction registrations). They are not diagnostics for the
            // compiled source tree. Preserve only messages the manager binds to a source/module;
            // the ordinary AngelScript callback remains authoritative for its own global errors.
            const char kind = diagnostic->is_error ? 'E' : diagnostic->is_info ? 'I' : 'W';
            capture_diagnostic(section, diagnostic->row, diagnostic->col, kind, message);
        }
    } else {
        EnterCriticalSection(&g_cs);
        invalidate_capture();
        LeaveCriticalSection(&g_cs);
    }
    g_orig_manager_diagnostic(manager, filename, diagnostic);
}

static bool parse_sig(const char* text, uint8_t* pattern, bool* mask, size_t* length) {
    size_t count = 0;
    for (const char* p = text; *p;) {
        if (*p == ' ') { ++p; continue; }
        if (count >= 64) return false;
        if (*p == '?') {
            pattern[count] = 0;
            mask[count++] = false;
            p += p[1] == '?' ? 2 : 1;
            continue;
        }
        unsigned value = 0;
        if (sscanf(p, "%2x", &value) != 1) return false;
        pattern[count] = static_cast<uint8_t>(value);
        mask[count++] = true;
        p += 2;
    }
    *length = count;
    return count > 0;
}

static bool get_text(uint8_t** base, size_t* size) {
    uint8_t* image = reinterpret_cast<uint8_t*>(GetModuleHandleW(nullptr));
    if (!readable(image, sizeof(IMAGE_DOS_HEADER))) return false;
    IMAGE_DOS_HEADER* dos = reinterpret_cast<IMAGE_DOS_HEADER*>(image);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0) return false;
    IMAGE_NT_HEADERS64* nt = reinterpret_cast<IMAGE_NT_HEADERS64*>(image + dos->e_lfanew);
    if (!readable(nt, sizeof *nt) || nt->Signature != IMAGE_NT_SIGNATURE) return false;
    if (nt->FileHeader.Machine != IMAGE_FILE_MACHINE_AMD64 ||
        nt->FileHeader.SizeOfOptionalHeader < sizeof(WORD) ||
        nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC)
        return false;
    IMAGE_SECTION_HEADER* section = IMAGE_FIRST_SECTION(nt);
    if (!readable(section, nt->FileHeader.NumberOfSections * sizeof *section)) return false;
    static const uint8_t text_name[IMAGE_SIZEOF_SHORT_NAME] = {
        '.', 't', 'e', 'x', 't', 0, 0, 0
    };
    for (int i = 0; i < nt->FileHeader.NumberOfSections; ++i) {
        if (!memcmp(section[i].Name, text_name, sizeof text_name)) {
            const size_t virtual_size = section[i].Misc.VirtualSize;
            const size_t raw_size = section[i].SizeOfRawData;
            // Rust scans the on-disk section while this helper scans the mapped image. Use the
            // same raw-backed intersection on both sides: exclude raw alignment padding beyond
            // VirtualSize and mapped zero-fill beyond SizeOfRawData.
            if (!virtual_size || !raw_size || raw_size > MAX_TEXT_BYTES) return false;
            const size_t scan_size = virtual_size < raw_size ? virtual_size : raw_size;
            if (!scan_size) return false;
            *base = image + section[i].VirtualAddress;
            *size = scan_size;
            return readable(*base, *size);
        }
    }
    return false;
}

static uint8_t* find_unique_sig(const char* signature, uint8_t* base, size_t size, int* count) {
    uint8_t pattern[64];
    bool mask[64];
    size_t length = 0;
    *count = 0;
    if (!parse_sig(signature, pattern, mask, &length)) return nullptr;
    uint8_t* found = nullptr;
    for (size_t i = 0; i + length <= size; ++i) {
        size_t j = 0;
        for (; j < length; ++j)
            if (mask[j] && base[i + j] != pattern[j]) break;
        if (j == length) {
            found = base + i;
            if (++*count > 1) return nullptr;
        }
    }
    return *count == 1 ? found : nullptr;
}

static bool body_shape_matches(uint8_t* body, size_t span, const ShapeClause* clauses,
                               size_t clause_count, uint8_t* text, size_t text_size) {
    const uintptr_t callback_addr = reinterpret_cast<uintptr_t>(body);
    const uintptr_t text_addr = reinterpret_cast<uintptr_t>(text);
    if (!body || !text || callback_addr < text_addr) return false;
    const uintptr_t relative_addr = callback_addr - text_addr;
    const size_t relative = static_cast<size_t>(relative_addr);
    if (relative > text_size || span > text_size - relative) return false;

    for (size_t clause_index = 0; clause_index < clause_count; ++clause_index) {
        const ShapeClause& clause = clauses[clause_index];
        uint8_t pattern[64];
        bool mask[64];
        size_t length = 0;
        if (!parse_sig(clause.signature, pattern, mask, &length) ||
            clause.offset > span || length > span - clause.offset)
            return false;
        const uint8_t* actual = body + clause.offset;
        for (size_t i = 0; i < length; ++i)
            if (mask[i] && actual[i] != pattern[i]) return false;
    }
    return true;
}

static bool callback_shape_matches(uint8_t* callback, uint8_t* text, size_t text_size) {
    return body_shape_matches(callback, kCallbackShapeSpan, kCallbackShape,
        sizeof kCallbackShape / sizeof kCallbackShape[0], text, text_size);
}

static bool manager_diagnostic_shape_matches(uint8_t* manager, uint8_t* text, size_t text_size) {
    return body_shape_matches(manager, kManagerDiagnosticShapeSpan, kManagerDiagnosticShape,
        sizeof kManagerDiagnosticShape / sizeof kManagerDiagnosticShape[0], text, text_size);
}

static DWORD WINAPI init_thread(LPVOID) {
    InitializeCriticalSection(&g_cs);
    uint8_t* text = nullptr;
    size_t text_size = 0;
    if (!get_text(&text, &text_size)) {
        report_status("unavailable: no readable .text section");
        return 0;
    }
    int count = 0;
    uint8_t* callback = find_unique_sig(kLogErrSig, text, text_size, &count);
    if (!callback) {
        char status[96];
        snprintf(status, sizeof status, "unavailable: signature matches=%d", count);
        report_status(status);
        return 0;
    }
    if (!callback_shape_matches(callback, text, text_size)) {
        report_status("unavailable: callback shape mismatch");
        return 0;
    }
    int manager_count = 0;
    uint8_t* manager = find_unique_sig(kManagerDiagnosticSig, text, text_size, &manager_count);
    if (!manager) {
        char status[96];
        snprintf(status, sizeof status, "unavailable: manager signature matches=%d", manager_count);
        report_status(status);
        return 0;
    }
    if (!manager_diagnostic_shape_matches(manager, text, text_size)) {
        report_status("unavailable: manager diagnostic shape mismatch");
        return 0;
    }
    if (MH_Initialize() != MH_OK ||
        MH_CreateHook(callback, reinterpret_cast<LPVOID>(&detour_logerr),
                      reinterpret_cast<LPVOID*>(&g_orig_logerr)) != MH_OK ||
        MH_CreateHook(manager, reinterpret_cast<LPVOID>(&detour_manager_diagnostic),
                      reinterpret_cast<LPVOID*>(&g_orig_manager_diagnostic)) != MH_OK ||
        MH_QueueEnableHook(callback) != MH_OK || MH_QueueEnableHook(manager) != MH_OK ||
        MH_ApplyQueued() != MH_OK) {
        report_status("unavailable: diagnostics hook initialization failed");
        return 0;
    }
    report_status("ready");
    return 0;
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(instance);
        HANDLE thread = CreateThread(nullptr, 0, init_thread, nullptr, 0, nullptr);
        if (thread) CloseHandle(thread);
    }
    return TRUE;
}
