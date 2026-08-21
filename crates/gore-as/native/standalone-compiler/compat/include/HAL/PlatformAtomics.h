#pragma once

#include <intrin.h>

struct FPlatformAtomics final {
    static long InterlockedIncrement(volatile long* value) noexcept {
        return _InterlockedIncrement(value);
    }
    static long InterlockedDecrement(volatile long* value) noexcept {
        return _InterlockedDecrement(value);
    }
    static int InterlockedIncrement(volatile int* value) noexcept {
        return static_cast<int>(_InterlockedIncrement(reinterpret_cast<volatile long*>(value)));
    }
    static int InterlockedDecrement(volatile int* value) noexcept {
        return static_cast<int>(_InterlockedDecrement(reinterpret_cast<volatile long*>(value)));
    }
};
