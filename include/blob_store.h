#pragma once

#include <array>
#include <cstdint>

namespace blob_store {
using key_t = std::array<std::uint8_t, 20>;
} // namespace blob_store