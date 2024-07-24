#pragma once

#include "local_file_system.rs.h"
#include <cstdint>
#include <memory>
#include <type_traits>

namespace blob_store {
using key_t = std::uint64_t;
// local store is backed by local file system, and it's thread-safe
using local_store_ref_inner =
    std::invoke_result_t<decltype(local_fs::blob_store_connect), std::string>;
using local_store_ref = std::shared_ptr<local_store_ref_inner>;
inline auto
connect_to_local_store(const std::string &root_dir) -> local_store_ref {
  return std::make_shared<local_store_ref_inner>(
      local_fs::blob_store_connect(root_dir));
};
} // namespace blob_store