#include "blob_store.h"
#include "local_file_system.rs.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <iostream>
#include <iterator>
#include <limits>
#include <random>
#include <vector>

int main() {
  using namespace blob_store::local_fs;

  using blob_store::key_t;
  using blob_store::local_fs::blob_store_t;

  constexpr std::size_t VALUE_SIZE{1024};
  auto store = blob_store::local_fs::blob_store_connect("./var/tmp-dev");
  auto key = key_t{};
  auto value = std::vector<uint8_t>{};
  value.reserve(VALUE_SIZE);
  std::random_device rd{};
  std::mt19937 gen{rd()};
  std::uniform_int_distribution<std::uint8_t> distrib{
      std::numeric_limits<std::uint8_t>::min(),
      std::numeric_limits<std::uint8_t>::max()};
  std::generate_n(std::back_inserter(value), VALUE_SIZE,
                  [&]() { return distrib(gen); });
  std::generate(key.begin(), key.end(), [&]() { return distrib(gen); });
  {
    std::cout << "put blob" << std::endl;
    auto slice = rust::Slice<const uint8_t>{value.data(), value.size()};
    store->create(key, slice);
  }
  {
    std::cout << "check existence" << std::endl;
    auto contains = store->contains(key);
    assert(contains);
  }
  {
    std::cout << "check meta" << std::endl;
    auto size = store->blob_size(key);
    assert(size == VALUE_SIZE);
  }
  {
    std::cout << "get blob" << std::endl;
    auto value2 = std::vector<uint8_t>(VALUE_SIZE);
    auto slice = rust::Slice<uint8_t>{value2.data(), value2.size()};
    store->get_all(key, slice);
    assert(value == value2);
  }
  return 0;
}