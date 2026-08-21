#pragma once

#include "CoreTypes.h"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdio>
#include <malloc.h>
#include <new>
#include <string>

#ifndef FORCEINLINE
#define FORCEINLINE __forceinline
#endif

#ifndef STDCALL
#define STDCALL __stdcall
#endif

#ifndef TEXT
#define TEXT(value) value
#endif

#ifndef ANSI_TO_TCHAR
#define ANSI_TO_TCHAR(value) value
#endif

#ifndef WITH_EDITOR
#define WITH_EDITOR 0
#endif

#ifndef UE_BUILD_SHIPPING
#define UE_BUILD_SHIPPING 1
#endif

#ifndef DO_BLUEPRINT_GUARD
#define DO_BLUEPRINT_GUARD 0
#endif

#ifndef AS_REFERENCE_DEBUGGING
#define AS_REFERENCE_DEBUGGING 0
#endif

#ifndef check
#define check(expression) assert(expression);
#endif

#ifndef checkSlow
#define checkSlow(expression) assert(expression);
#endif

#ifndef ensureMsgf
#define ensureMsgf(expression, format, ...) \
    ((expression) ? true : (std::fprintf(stderr, "%s\n", format), false))
#endif

struct FMemory final {
    static void* Malloc(const std::size_t size, std::size_t alignment = alignof(std::max_align_t)) {
        alignment = (std::max)(alignment, sizeof(void*));
        void* result = _aligned_malloc((std::max)(size, std::size_t{1}), alignment);
        if (result == nullptr) {
            throw std::bad_alloc{};
        }
        return result;
    }
    static void Free(void* pointer) noexcept { _aligned_free(pointer); }
    static void* Memcpy(void* destination, const void* source, const std::size_t size) noexcept {
        return std::memcpy(destination, source, size);
    }
};

inline constexpr bool GIsEditor = false;

class FMemStackBase final {
public:
    FMemStackBase() = default;
    ~FMemStackBase() {
        for (void* allocation : allocations_) {
            FMemory::Free(allocation);
        }
    }
    FMemStackBase(const FMemStackBase&) = delete;
    FMemStackBase& operator=(const FMemStackBase&) = delete;

    void* Alloc(const std::size_t size, const std::size_t alignment) {
        void* result = FMemory::Malloc(size, alignment);
        allocations_.push_back(result);
        return result;
    }

private:
    std::vector<void*> allocations_;
};

struct FMath final {
    template <typename ValueType>
    static constexpr const ValueType& Max(const ValueType& left, const ValueType& right) {
        return (std::max)(left, right);
    }
    template <typename ValueType>
    static constexpr ValueType Min3(
        const ValueType& first, const ValueType& second, const ValueType& third) {
        return (std::min)(first, (std::min)(second, third));
    }
    static float Pow(const float base, const float exponent) { return std::pow(base, exponent); }
    static double Pow(const double base, const double exponent) { return std::pow(base, exponent); }
};

struct FCrc final {
    static uint32 Strihash_DEPRECATED(const int32 length, const char* text) noexcept {
        uint32 hash = 0x811c9dc5U;
        for (int32 index = 0; index < length; ++index) {
            unsigned char value = static_cast<unsigned char>(text[index]);
            if (value >= 'A' && value <= 'Z') {
                value = static_cast<unsigned char>(value - 'A' + 'a');
            }
            hash ^= value;
            hash *= 16777619U;
        }
        return hash;
    }
};
