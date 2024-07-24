#include <algorithm>
#include <cassert>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <getopt.h>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <random>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "blob_store.h"
#include "local_file_system.rs.h"
#include "memmap.rs.h"
#include "sqlite.rs.h"

constexpr std::string_view help_message = R"(\
USAGE: bench <Device> <Load> <Size>
    Device:   Set store device path
    Load:     Set test load
    Size:     Set blob size(in KB))";

#ifdef _ENABLE_SQLITE
constexpr bool ENABLE_SQLITE = true;
#else
constexpr bool ENABLE_SQLITE = false;
#endif

auto main(int argc, char **argv) -> int {
  if (argc != 4) {
    std::cerr << help_message << std::endl;
    return -1;
  }
  std::string dev_path{argv[1]};
  std::size_t load{std::stoul(argv[2])};
  std::size_t blob_size{std::stoul(argv[3])};
  std::cout << "Device: " << dev_path << std::endl;
  std::cout << "Load: " << load << std::endl;
  std::cout << "Blob size: " << blob_size << "KB" << std::endl;
  blob_size <<= 10;

  if (dev_path.empty()) {
    std::cerr << "Please specify the store device path" << std::endl;
    return -1;
  }
  if (!std::filesystem::exists(dev_path)) {
    std::cerr << "Store device path does not exist" << std::endl;
    return -1;
  }

  auto key_values =
      std::vector<std::pair<blob_store::key_t, std::vector<std::uint8_t>>>{};
  key_values.reserve(load);
  std::random_device rd{};
  std::mt19937 gen{rd()};
  std::uniform_int_distribution<std::uint8_t> distrib{
      std::numeric_limits<std::uint8_t>::min(),
      std::numeric_limits<std::uint8_t>::max()};
  // local fs
  {
    auto path = std::filesystem::path{dev_path} / "local_fs";
    std::filesystem::create_directory(path);
    using namespace blob_store::local_fs;
    auto store = blob_store_connect(path);
    auto put_elapsed = std::chrono::nanoseconds::zero();
    auto get_elapsed = std::chrono::nanoseconds::zero();
    {
      // put blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      for (std::size_t i = 0; i < load; i++) {
        auto key = blob_store::key_t{};
        auto value = std::vector<uint8_t>{};
        value.reserve(blob_size);
        std::generate_n(std::back_inserter(value), blob_size,
                        [&]() { return distrib(gen); });
        std::generate_n(reinterpret_cast<uint8_t *>(&key), sizeof(key),
                        [&]() { return distrib(gen); });
        store->create(key, {value.data(), value.size()});
        key_values.emplace_back(key, std::move(value));
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      put_elapsed = end - start;
    }
    {
      // get blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      auto value2 = std::vector<uint8_t>(blob_size);
      value2.resize(blob_size);
      for (auto &[key, value] : key_values) {
        store->get_all(key, {value2.data(), value2.size()});
        assert(value2 == value);
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      get_elapsed = end - start;
    }
    key_values.clear();
    std::cout << "Local fs put elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     put_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "Local fs get elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     get_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "throughput: " << std::fixed << std::setprecision(4)
              << static_cast<double>(load * (blob_size >> 10)) /
                     (std::chrono::duration_cast<std::chrono::milliseconds>(
                          put_elapsed + get_elapsed)
                          .count())
              << "MB/ms" << std::endl;
  }
  // memmap
  {
    auto path = std::filesystem::path{dev_path} / "memmap";
    std::filesystem::create_directory(path);
    using namespace blob_store::memmap;
    auto store = blob_store_connect(path);
    auto put_elapsed = std::chrono::nanoseconds::zero();
    auto get_elapsed = std::chrono::nanoseconds::zero();
    {
      // put blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      for (std::size_t i = 0; i < load; i++) {
        auto key = blob_store::key_t{};
        auto value = std::vector<uint8_t>{};
        value.reserve(blob_size);
        std::generate_n(std::back_inserter(value), blob_size,
                        [&]() { return distrib(gen); });

        std::generate_n(reinterpret_cast<uint8_t *>(&key), sizeof(key),
                        [&]() { return distrib(gen); });
        store->create(key, {value.data(), value.size()});
        key_values.emplace_back(key, std::move(value));
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      put_elapsed = end - start;
    }
    {
      // get blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      auto value2 = std::vector<uint8_t>(blob_size);
      value2.resize(blob_size);
      for (auto &[key, value] : key_values) {
        store->get_all(key, {value2.data(), value2.size()});
        assert(value2 == value);
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      get_elapsed = end - start;
    }
    key_values.clear();
    std::cout << "Memmap put elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     put_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "Memmap get elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     get_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "throughput: "
              << static_cast<double>(load * (blob_size >> 10)) /
                     (std::chrono::duration_cast<std::chrono::milliseconds>(
                          put_elapsed + get_elapsed)
                          .count())
              << "MB/ms" << std::endl;
  }
  // sqlite
  if constexpr (ENABLE_SQLITE) {
    auto path = std::filesystem::path{dev_path} / "sqlite";
    std::filesystem::create_directory(path);
    using namespace blob_store::sqlite;
    auto store = blob_store_connect(path);
    auto put_elapsed = std::chrono::nanoseconds::zero();
    auto get_elapsed = std::chrono::nanoseconds::zero();
    {
      // put blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      for (std::size_t i = 0; i < load; i++) {
        auto key = blob_store::key_t{};
        auto value = std::vector<uint8_t>{};
        value.reserve(blob_size);
        std::generate_n(std::back_inserter(value), blob_size,
                        [&]() { return distrib(gen); });
        std::generate_n(reinterpret_cast<uint8_t *>(&key), sizeof(key),
                        [&]() { return distrib(gen); });
        store->create(key, {value.data(), value.size()});
        key_values.emplace_back(key, std::move(value));
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      put_elapsed = end - start;
    }
    {
      // get blobs
      std::chrono::steady_clock::time_point start =
          std::chrono::steady_clock::now();
      auto value2 = std::vector<uint8_t>(blob_size);
      value2.resize(blob_size);
      for (auto &[key, value] : key_values) {
        store->get_all(key, {value2.data(), value2.size()});
        assert(value2 == value);
      }
      std::chrono::steady_clock::time_point end =
          std::chrono::steady_clock::now();
      get_elapsed = end - start;
    }
    key_values.clear();
    std::cout << "Sqlite put elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     put_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "Sqlite get elapsed: "
              << std::chrono::duration_cast<std::chrono::milliseconds>(
                     get_elapsed)
                     .count()
              << "ms" << std::endl;
    std::cout << "throughput: "
              << static_cast<double>(load * (blob_size >> 10)) /
                     (std::chrono::duration_cast<std::chrono::milliseconds>(
                          put_elapsed + get_elapsed)
                          .count())
              << "MB/ms" << std::endl;
  }
}
