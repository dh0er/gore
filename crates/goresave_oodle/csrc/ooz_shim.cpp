// Clean C ABI over the vendored ooz codec. Kraken only on the encode side;
// decode dispatches every Oodle codec.
#include "stdafx.h" // ooz precompiled header: provides uint8/byte typedefs + simde
#include <cstddef>
#include "compress.h"

// Defined in kraken.cpp, not declared in any header.
int Kraken_Decompress(const unsigned char* src, size_t src_len,
                      unsigned char* dst, size_t dst_len);

extern "C" {

// Returns decoded byte count, or < 0 on failure.
int goresave_ooz_decompress(const unsigned char* src, size_t src_len,
                            unsigned char* dst, size_t dst_len) {
    return Kraken_Decompress(src, src_len, dst, dst_len);
}

// Kraken encode (codec id 8). Returns compressed byte count, or <= 0 on failure.
int goresave_ooz_compress_kraken(const unsigned char* src, int src_len,
                                 unsigned char* dst, int level) {
    return CompressBlock(8, const_cast<uint8*>(reinterpret_cast<const uint8*>(src)),
                         reinterpret_cast<uint8*>(dst), src_len, level,
                         nullptr, nullptr, nullptr);
}

} // extern "C"
