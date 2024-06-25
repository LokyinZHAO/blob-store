#include "blob_store.h"
#include "local_file_system.rs.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <iostream>
#include <iterator>
#include <limits>
#include <random>
#include <string_view>
#include <vector>

constexpr std::string_view help_message = R"(USAGE: example <Device>)";

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << help_message << std::endl;
    return -1;
  }
  std::filesystem::path dev_path{argv[1]};

  using namespace blob_store::local_fs;

  using blob_store::key_t;
  using blob_store::local_fs::blob_store_t;

  // connect to blob store
  constexpr std::size_t VALUE_SIZE{1024};
  auto store = blob_store::local_fs::blob_store_connect(dev_path);
  // generate random key and value
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
    auto slice2 = rust::Slice<uint8_t>{value2.data(), value2.size()};
    store->get_all(key, slice2);
    assert(value == value2);
    // get range VALUE_SIZE / 3 .. VALUE_SIZE / 3 * 2
    auto value3 = std::vector<uint8_t>(VALUE_SIZE / 3);
    auto slice3 = rust::Slice<uint8_t>{value3.data(), value3.size()};
    store->get_offset(key, slice3, VALUE_SIZE / 3);
    assert(std::equal(value.begin() + VALUE_SIZE / 3,
                      value.begin() + VALUE_SIZE / 3 * 2, value3.begin()));
  }
  {
    std::cout << "remove blob" << std::endl;
    store->remove(key);
    assert(!store->contains(key));
  }
  return 0;
}