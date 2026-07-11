// Temporary G1R AngelScript diagnostic capture helper.
//
// Hooks only the UE-AngelScript per-message callback. The Rust launcher independently scans this
// same masked AOB offline and injects the DLL only when exactly one match exists. The DLL repeats
// that uniqueness check in the mapped .text section and reports `ready` only after MinHook enables
// the callback. No game file is patched and no DLL is installed permanently.

#include <windows.h>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include "MinHook.h"

// LogAngelscriptError(asSMessageInfo*, void*). Observed at RVA 0x467f5b0 in the 2026-07-10
// hotfix (prior build: 0x467e200); RVAs are documentation only and never used for lookup.
static const char* kLogErrSig =
    "40 55 56 57 48 8D AC 24 60 FF FF FF 48 81 EC A0 01 00 00 48 8B 05 ?? ?? ?? ?? "
    "48 33 C4 48 89 85 80 00 00 00 8B 15 ?? ??";

struct asMsgInfo {
    const char* section;
    int32_t row;
    int32_t col;
    int32_t type;
    const char* message;
};

typedef void (*LogErr_t)(void*, void*);
static LogErr_t g_orig_logerr = nullptr;
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

static void detour_logerr(void* msgp, void* data) {
    if (readable(msgp, sizeof(asMsgInfo))) {
        asMsgInfo* msg = static_cast<asMsgInfo*>(msgp);
        char section[2048], text[8192], line[10240];
        copy_cstr(msg->section, section, sizeof section);
        copy_cstr(msg->message, text, sizeof text);
        // Per-function progress dominates the 7,305-module build and can consume the entire
        // bounded capture before a real warning/error arrives. It is routine information, not a
        // diagnostic; filter it before opening/writing the FILE_FLAG_WRITE_THROUGH stream.
        const bool routine_compile = msg->type == 2 && !strncmp(text, "Compiling ", 10);
        if (text[0] && !routine_compile) {
            EnterCriticalSection(&g_cs);
            if (g_errh == INVALID_HANDLE_VALUE) {
                wchar_t pathw[MAX_PATH];
                DWORD n = GetEnvironmentVariableW(L"GORE_AS_ERRFILE", pathw, MAX_PATH);
                if (n > 0 && n < MAX_PATH) {
                    g_errh = CreateFileW(pathw, FILE_APPEND_DATA,
                        FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_ALWAYS,
                        FILE_FLAG_WRITE_THROUGH, nullptr);
                }
            }
            if (g_errh != INVALID_HANDLE_VALUE) {
                int length;
                if (strcmp(section, g_last_section) != 0) {
                    length = snprintf(line, sizeof line, "=== %s ===\n",
                                      section[0] ? section : "(?)");
                    if (length >= static_cast<int>(sizeof line)) length = sizeof line - 1;
                    capture_write(line, length);
                    strncpy(g_last_section, section, sizeof g_last_section - 1);
                    g_last_section[sizeof g_last_section - 1] = 0;
                }
                // AngelScript asSMessageInfo: 0=error, 1=warning, 2=information.
                char kind = msg->type == 0 ? 'E' : msg->type == 1 ? 'W' : msg->type == 2 ? 'I' : '?';
                if (msg->row || msg->col)
                    length = snprintf(line, sizeof line, "(%d:%d) [%c] %s\n",
                                      msg->row, msg->col, kind, text);
                else
                    length = snprintf(line, sizeof line, "[%c] %s\n", kind, text);
                if (length >= static_cast<int>(sizeof line)) length = sizeof line - 1;
                capture_write(line, length);
            }
            LeaveCriticalSection(&g_cs);
        }
    }
    g_orig_logerr(msgp, data);
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
    if (MH_Initialize() != MH_OK ||
        MH_CreateHook(callback, reinterpret_cast<LPVOID>(&detour_logerr),
                      reinterpret_cast<LPVOID*>(&g_orig_logerr)) != MH_OK ||
        MH_EnableHook(callback) != MH_OK) {
        report_status("unavailable: callback hook initialization failed");
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
