#include "materializer.hpp"

#include <filesystem>
#include <iostream>

int wmain(const int argc, wchar_t** argv) {
  if (argc != 3) {
    std::wcerr << L"usage: gore_as_capture_materializer.exe <sealed.capture> <new-summary.json>\n";
    return 2;
  }
  const auto result = gore_as_capture::v1::offline::materialize_capture_summary_v1(
      std::filesystem::path(argv[1]), std::filesystem::path(argv[2]));
  if (result.error != gore_as_capture::v1::offline::MaterializeError::ok) {
    std::cerr << "capture materialization failed: "
              << gore_as_capture::v1::offline::materialize_error_name(result.error) << '\n';
    return 1;
  }
  std::cout << "materialized " << result.record_count << " sealed records\n";
  return 0;
}
