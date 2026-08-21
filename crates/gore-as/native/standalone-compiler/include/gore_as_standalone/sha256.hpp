#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>

namespace gore::as::standalone {

using sha256_digest = std::array<std::uint8_t, 32U>;

class sha256 final {
public:
    sha256() noexcept;
    void update(const void* bytes, std::size_t size) noexcept;
    void update(std::string_view bytes) noexcept { update(bytes.data(), bytes.size()); }
    [[nodiscard]] sha256_digest finish() noexcept;

private:
    void transform(const std::uint8_t* block) noexcept;
    std::array<std::uint32_t, 8U> state_{};
    std::array<std::uint8_t, 64U> block_{};
    std::uint64_t total_bytes_ = 0U;
    std::size_t block_bytes_ = 0U;
    bool finished_ = false;
};

[[nodiscard]] sha256_digest sha256_bytes(const void* bytes, std::size_t size) noexcept;
[[nodiscard]] std::string sha256_hex(const sha256_digest& digest);
bool parse_sha256_hex(std::string_view text, sha256_digest& output) noexcept;

} // namespace gore::as::standalone
