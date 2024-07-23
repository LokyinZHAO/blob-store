#pragma once

#include "local_file_system.rs.h"
#include <cstdint>
#include <memory>

namespace blob_store {
using key_t = std::uint64_t;
// local store is backed by local file system, and it's thread-safe
using local_store_ref = std::shared_ptr<blob_store::local_fs::blob_store_t>;
inline auto
connect_to_local_store(const std::string &root_dir) -> local_store_ref {
  return std::make_shared<blob_store::local_fs::blob_store_t>(
      local_fs::blob_store_connect(root_dir));
};
} // namespace blob_store