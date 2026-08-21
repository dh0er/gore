#include "gore_as_standalone/sha256.hpp"

#include <algorithm>
#include <cstring>

namespace gore::as::standalone {
namespace {

constexpr std::array<std::uint32_t, 64U> constants{{
    0x428a2f98U, 0x71374491U, 0xb5c0fbcfU, 0xe9b5dba5U, 0x3956c25bU, 0x59f111f1U,
    0x923f82a4U, 0xab1c5ed5U, 0xd807aa98U, 0x12835b01U, 0x243185beU, 0x550c7dc3U,
    0x72be5d74U, 0x80deb1feU, 0x9bdc06a7U, 0xc19bf174U, 0xe49b69c1U, 0xefbe4786U,
    0x0fc19dc6U, 0x240ca1ccU, 0x2de92c6fU, 0x4a7484aaU, 0x5cb0a9dcU, 0x76f988daU,
    0x983e5152U, 0xa831c66dU, 0xb00327c8U, 0xbf597fc7U, 0xc6e00bf3U, 0xd5a79147U,
    0x06ca6351U, 0x14292967U, 0x27b70a85U, 0x2e1b2138U, 0x4d2c6dfcU, 0x53380d13U,
    0x650a7354U, 0x766a0abbU, 0x81c2c92eU, 0x92722c85U, 0xa2bfe8a1U, 0xa81a664bU,
    0xc24b8b70U, 0xc76c51a3U, 0xd192e819U, 0xd6990624U, 0xf40e3585U, 0x106aa070U,
    0x19a4c116U, 0x1e376c08U, 0x2748774cU, 0x34b0bcb5U, 0x391c0cb3U, 0x4ed8aa4aU,
    0x5b9cca4fU, 0x682e6ff3U, 0x748f82eeU, 0x78a5636fU, 0x84c87814U, 0x8cc70208U,
    0x90befffaU, 0xa4506cebU, 0xbef9a3f7U, 0xc67178f2U,
}};

constexpr std::uint32_t rotate_right(const std::uint32_t value, const unsigned count) noexcept {
    return (value >> count) | (value << (32U - count));
}

} // namespace

sha256::sha256() noexcept
    : state_{{0x6a09e667U, 0xbb67ae85U, 0x3c6ef372U, 0xa54ff53aU,
              0x510e527fU, 0x9b05688cU, 0x1f83d9abU, 0x5be0cd19U}} {}

void sha256::transform(const std::uint8_t* const block) noexcept {
    std::array<std::uint32_t, 64U> words{};
    for (std::size_t index = 0U; index < 16U; ++index) {
        const std::size_t offset = index * 4U;
        words[index] = (static_cast<std::uint32_t>(block[offset]) << 24U) |
            (static_cast<std::uint32_t>(block[offset + 1U]) << 16U) |
            (static_cast<std::uint32_t>(block[offset + 2U]) << 8U) |
            static_cast<std::uint32_t>(block[offset + 3U]);
    }
    for (std::size_t index = 16U; index < words.size(); ++index) {
        const std::uint32_t s0 = rotate_right(words[index - 15U], 7U) ^
            rotate_right(words[index - 15U], 18U) ^ (words[index - 15U] >> 3U);
        const std::uint32_t s1 = rotate_right(words[index - 2U], 17U) ^
            rotate_right(words[index - 2U], 19U) ^ (words[index - 2U] >> 10U);
        words[index] = words[index - 16U] + s0 + words[index - 7U] + s1;
    }

    std::uint32_t a = state_[0]; std::uint32_t b = state_[1];
    std::uint32_t c = state_[2]; std::uint32_t d = state_[3];
    std::uint32_t e = state_[4]; std::uint32_t f = state_[5];
    std::uint32_t g = state_[6]; std::uint32_t h = state_[7];
    for (std::size_t index = 0U; index < words.size(); ++index) {
        const std::uint32_t sum1 = rotate_right(e, 6U) ^ rotate_right(e, 11U) ^ rotate_right(e, 25U);
        const std::uint32_t choose = (e & f) ^ ((~e) & g);
        const std::uint32_t temp1 = h + sum1 + choose + constants[index] + words[index];
        const std::uint32_t sum0 = rotate_right(a, 2U) ^ rotate_right(a, 13U) ^ rotate_right(a, 22U);
        const std::uint32_t majority = (a & b) ^ (a & c) ^ (b & c);
        const std::uint32_t temp2 = sum0 + majority;
        h = g; g = f; f = e; e = d + temp1; d = c; c = b; b = a; a = temp1 + temp2;
    }
    state_[0] += a; state_[1] += b; state_[2] += c; state_[3] += d;
    state_[4] += e; state_[5] += f; state_[6] += g; state_[7] += h;
}

void sha256::update(const void* const bytes, const std::size_t size) noexcept {
    if (finished_ || size == 0U) return;
    const auto* input = static_cast<const std::uint8_t*>(bytes);
    total_bytes_ += static_cast<std::uint64_t>(size);
    std::size_t remaining = size;
    while (remaining != 0U) {
        const std::size_t copied = std::min(remaining, block_.size() - block_bytes_);
        std::memcpy(block_.data() + block_bytes_, input, copied);
        block_bytes_ += copied;
        input += copied;
        remaining -= copied;
        if (block_bytes_ == block_.size()) {
            transform(block_.data());
            block_bytes_ = 0U;
        }
    }
}

sha256_digest sha256::finish() noexcept {
    if (!finished_) {
        const std::uint64_t bit_count = total_bytes_ * 8U;
        block_[block_bytes_++] = 0x80U;
        if (block_bytes_ > 56U) {
            std::fill(block_.begin() + static_cast<std::ptrdiff_t>(block_bytes_), block_.end(), 0U);
            transform(block_.data());
            block_bytes_ = 0U;
        }
        std::fill(block_.begin() + static_cast<std::ptrdiff_t>(block_bytes_), block_.begin() + 56, 0U);
        for (std::size_t index = 0U; index < 8U; ++index) {
            block_[63U - index] = static_cast<std::uint8_t>(bit_count >> (index * 8U));
        }
        transform(block_.data());
        finished_ = true;
    }
    sha256_digest output{};
    for (std::size_t index = 0U; index < state_.size(); ++index) {
        output[index * 4U] = static_cast<std::uint8_t>(state_[index] >> 24U);
        output[index * 4U + 1U] = static_cast<std::uint8_t>(state_[index] >> 16U);
        output[index * 4U + 2U] = static_cast<std::uint8_t>(state_[index] >> 8U);
        output[index * 4U + 3U] = static_cast<std::uint8_t>(state_[index]);
    }
    return output;
}

sha256_digest sha256_bytes(const void* const bytes, const std::size_t size) noexcept {
    sha256 hash;
    hash.update(bytes, size);
    return hash.finish();
}

std::string sha256_hex(const sha256_digest& digest) {
    constexpr char hex[] = "0123456789abcdef";
    std::string output;
    output.resize(digest.size() * 2U);
    for (std::size_t index = 0U; index < digest.size(); ++index) {
        output[index * 2U] = hex[digest[index] >> 4U];
        output[index * 2U + 1U] = hex[digest[index] & 0x0fU];
    }
    return output;
}

bool parse_sha256_hex(const std::string_view text, sha256_digest& output) noexcept {
    if (text.size() != output.size() * 2U) return false;
    const auto digit = [](const char ch) -> int {
        if (ch >= '0' && ch <= '9') return ch - '0';
        if (ch >= 'a' && ch <= 'f') return ch - 'a' + 10;
        return -1;
    };
    sha256_digest staged{};
    for (std::size_t index = 0U; index < staged.size(); ++index) {
        const int high = digit(text[index * 2U]);
        const int low = digit(text[index * 2U + 1U]);
        if (high < 0 || low < 0) return false;
        staged[index] = static_cast<std::uint8_t>((high << 4U) | low);
    }
    output = staged;
    return true;
}

} // namespace gore::as::standalone
