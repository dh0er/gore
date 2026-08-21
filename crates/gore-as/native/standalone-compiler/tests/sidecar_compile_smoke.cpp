#include "gore_as_standalone/precompiled_data.hpp"
#include "gore_as_standalone/sha256.hpp"
#include "gore_as_standalone/sidecar_compile.hpp"

#include "angelscript.h"

#include <Windows.h>

#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

namespace standalone = gore::as::standalone;
namespace precompiled = gore::as::standalone::precompiled;

namespace {

std::string fixed_operations(const unsigned size, const unsigned alignment) {
    return "{\"can_be_template_subtype\":true,\"can_construct\":true,\"need_construct\":false,"
        "\"can_destruct\":true,\"need_destruct\":false,\"can_copy\":true,\"need_copy\":false,"
        "\"can_compare\":true,\"can_hash_value\":true,\"value_size\":" + std::to_string(size) +
        ",\"value_alignment\":" + std::to_string(alignment) + ",\"is_object_pointer\":false}";
}

std::string json_path(const std::filesystem::path& path) {
    std::string text = path.generic_string();
    std::string escaped;
    for (const char ch : text) {
        if (ch == '"' || ch == '\\') escaped.push_back('\\');
        escaped.push_back(ch);
    }
    return escaped;
}

bool write_bytes(const std::filesystem::path& path, const std::vector<std::uint8_t>& bytes) {
    std::ofstream stream(path, std::ios::binary | std::ios::trunc);
    stream.write(reinterpret_cast<const char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
    return stream.good();
}

bool write_text(const std::filesystem::path& path, const std::string& text) {
    return write_bytes(path, {text.begin(), text.end()});
}

std::string blob(const std::string& path, const std::string& bytes) {
    return "{\"path\":\"" + path + "\",\"byte_len\":" + std::to_string(bytes.size()) +
        ",\"sha256\":\"" + standalone::sha256_hex(
            standalone::sha256_bytes(bytes.data(), bytes.size())) + "\"}";
}

std::string path_seal(const std::filesystem::path& path, const std::vector<std::uint8_t>& bytes) {
    return "{\"path\":\"" + json_path(path) + "\",\"byte_len\":" + std::to_string(bytes.size()) +
        ",\"sha256\":\"" + standalone::sha256_hex(
            standalone::sha256_bytes(bytes.data(), bytes.size())) + "\"}";
}

} // namespace

int main() {
    const std::filesystem::path root = std::filesystem::temp_directory_path() /
        ("gore-as-native-sidecar-smoke-" + std::to_string(GetCurrentProcessId()) + "-" +
         std::to_string(GetTickCount64()));
    const std::filesystem::path profile_root = root / "profile";
    const std::filesystem::path source_root = root / "sources";
    const std::filesystem::path output_root = root / "output";
    std::error_code filesystem_error;
    std::filesystem::create_directories(profile_root / "engine", filesystem_error);
    std::filesystem::create_directories(profile_root / "frontend", filesystem_error);
    std::filesystem::create_directories(source_root, filesystem_error);
    std::filesystem::create_directories(output_root, filesystem_error);
    if (filesystem_error) return 1;

    asIScriptEngine* const probe = asCreateScriptEngine(23300U);
    if (probe == nullptr || probe->SetEngineProperty(asEP_OPTIMIZE_BYTECODE, 1U) < 0) return 2;
    const int enum_id = probe->RegisterEnum("ETest");
    probe->ShutDownAndRelease();
    if (enum_id < 0) return 3;

    const std::string digest(64U, '1');
    const std::string properties =
        "{\"schema\":\"gore.as.engine-properties\",\"schema_version\":1,"
        "\"settings\":[{\"ordinal\":0,\"property\":\"optimize_bytecode\",\"value\":1}],"
        "\"canonical_sha256\":\"" + digest + "\"}";
    const std::array<std::pair<const char*, std::pair<unsigned, unsigned>>, 11U> primitives{{
        {"bool", {1, 1}}, {"int8", {1, 1}}, {"int16", {2, 2}}, {"int32", {4, 4}},
        {"int64", {8, 8}}, {"uint8", {1, 1}}, {"uint16", {2, 2}}, {"uint32", {4, 4}},
        {"uint64", {8, 8}}, {"float32", {4, 4}}, {"float64", {8, 8}},
    }};
    std::string primitive_json;
    for (std::size_t index = 0U; index < primitives.size(); ++index) {
        if (index != 0U) primitive_json.push_back(',');
        primitive_json += "{\"ordinal\":" + std::to_string(index) + ",\"primitive\":\"" +
            primitives[index].first + "\",\"operations\":" +
            fixed_operations(primitives[index].second.first, primitives[index].second.second) + "}";
    }
    const std::string trace =
        "{\"schema\":\"gore.as.registration-trace\",\"schema_version\":1,\"host_stubs\":[],"
        "\"primitive_operations\":[" + primitive_json + "],\"dynamic_script_operations\":{"
        "\"delegate\":" + fixed_operations(16, 8) + ",\"multicast_delegate\":" +
        fixed_operations(16, 8) + "},\"entries\":[{\"kind\":\"enum\",\"ordinal\":0,"
        "\"registration_id\":0,\"context\":{\"namespace\":\"\",\"config_group\":null,"
        "\"access_mask\":4294967295},\"type_id\":1,\"declaration\":\"ETest\","
        "\"type_operations\":{\"kind\":\"fixed\",\"operations\":" + fixed_operations(1, 1) +
        "}}],\"canonical_sha256\":\"" + digest + "\"}";
    const std::string snapshot =
        "{\"schema\":\"gore.as.post-bind-snapshot\",\"schema_version\":1,"
        "\"engine_properties_sha256\":\"" + digest + "\",\"registration_trace_sha256\":\"" +
        digest + "\",\"entries\":[{\"ordinal\":0,\"trace_registration_id\":0,"
        "\"result\":{\"kind\":\"enum\",\"engine_type_id\":" + std::to_string(enum_id) +
        "}}],\"final_states\":[],\"canonical_sha256\":\"" + digest + "\"}";
    const std::string preprocessor =
        "{\"schema\":\"gore.as.preprocessor-config\",\"schema_version\":1,"
        "\"automatic_imports\":true,\"warn_on_manual_import_statements\":true,"
        "\"use_editor_scripts\":false,\"effective_flags\":["
        "{\"ordinal\":0,\"name\":\"COOK_COMMANDLET\",\"value\":false},"
        "{\"ordinal\":1,\"name\":\"EDITOR\",\"value\":false},"
        "{\"ordinal\":2,\"name\":\"EDITORONLY_DATA\",\"value\":false},"
        "{\"ordinal\":3,\"name\":\"RELEASE\",\"value\":true},"
        "{\"ordinal\":4,\"name\":\"TEST\",\"value\":false},"
        "{\"ordinal\":5,\"name\":\"WITH_SERVER_CODE\",\"value\":true}],"
        "\"default_function_blueprint_callable\":true,"
        "\"default_property_edit_specifier\":\"edit_anywhere\","
        "\"default_property_edit_specifier_for_structs\":\"edit_anywhere\","
        "\"default_property_blueprint_specifier\":\"blueprint_read_write\","
        "\"static_class_mode\":\"allowed\",\"script_float_is_float64\":false,"
        "\"angelscript_haze\":false,\"enforce_server_rpc_validation\":false,"
        "\"blueprint_event_argument_specializations\":[],\"native_super_types\":[],"
        "\"canonical_sha256\":\"" + digest + "\"}";
    const std::string class_generator =
        "{\"schema\":\"gore.as.class-generator-config\",\"schema_version\":1,"
        "\"mark_non_uproperty_properties_as_transient\":false,\"canonical_sha256\":\"" + digest + "\"}";
    const std::string compiler_options =
        "{\"schema\":\"gore.as.compiler-options\",\"schema_version\":1,"
        "\"error_on_incorrect_editor_only_code\":false,"
        "\"warn_on_divergent_comparison_operator_overloads\":false,"
        "\"warn_on_implicit_signed_unsigned_conversion\":false,"
        "\"warn_on_increment_decrement_in_complex_expression\":false,"
        "\"warn_on_unused_return_value_for_const_methods\":false,\"canonical_sha256\":\"" + digest + "\"}";
    const std::string placeholder = "profile-blob";
    if (!write_text(profile_root / "engine/properties.json", properties) ||
        !write_text(profile_root / "engine/trace.json", trace) ||
        !write_text(profile_root / "engine/snapshot.json", snapshot) ||
        !write_text(profile_root / "frontend/preprocessor.json", preprocessor) ||
        !write_text(profile_root / "frontend/class-generator.json", class_generator) ||
        !write_text(profile_root / "frontend/compiler-options.json", compiler_options) ||
        !write_text(profile_root / "blob.bin", placeholder)) return 4;

    precompiled::cache empty_cache;
    empty_cache.build_identifier = 1;
    std::vector<std::uint8_t> base;
    precompiled::codec_error codec_error;
    if (!precompiled::encode(empty_cache, base, codec_error)) return 5;
    const std::vector<std::uint8_t> binds{'b', 'i', 'n', 'd', 's'};
    const std::string source = "int Answer() { return 42; }\n";
    const std::vector<std::uint8_t> source_bytes(source.begin(), source.end());
    const std::filesystem::path base_path = root / "base.cache";
    const std::filesystem::path binds_path = root / "binds.cache";
    const std::filesystem::path source_path = source_root / "Module.as";
    const std::filesystem::path output_path = output_root / "generated.cache";
    if (!write_bytes(base_path, base) || !write_bytes(binds_path, binds) ||
        !write_bytes(source_path, source_bytes)) return 6;

    const std::string file_seal =
        "{\"byte_len\":3,\"sha256\":\"" + digest + "\",\"steam_content_sha1\":\"" +
        std::string(40U, '2') + "\"}";
    const std::string base_oracle =
        "{\"byte_len\":" + std::to_string(base.size()) + ",\"sha256\":\"" +
        standalone::sha256_hex(standalone::sha256_bytes(base.data(), base.size())) +
        "\",\"steam_content_sha1\":\"" + std::string(40U, '2') + "\"}";
    const std::string binds_oracle =
        "{\"byte_len\":" + std::to_string(binds.size()) + ",\"sha256\":\"" +
        standalone::sha256_hex(standalone::sha256_bytes(binds.data(), binds.size())) +
        "\",\"steam_content_sha1\":\"" + std::string(40U, '2') + "\"}";
    const std::string depot_seal = "{\"byte_len\":3,\"sha256\":\"" + digest + "\"}";
    const std::string properties_blob = blob("engine/properties.json", properties);
    const std::string trace_blob = blob("engine/trace.json", trace);
    const std::string snapshot_blob = blob("engine/snapshot.json", snapshot);
    const std::string preprocessor_blob = blob("frontend/preprocessor.json", preprocessor);
    const std::string class_blob = blob("frontend/class-generator.json", class_generator);
    const std::string options_blob = blob("frontend/compiler-options.json", compiler_options);
    const std::string common_blob = blob("blob.bin", placeholder);
    const std::string profile_payload =
        "{\"schema\":\"gore.as.compiler-profile\",\"schema_version\":1,"
        "\"target\":{\"steam_app_id\":1297900,\"steam_build_id\":24539464,\"depot_id\":1297901,"
        "\"depot_manifest_gid\":1585071322101748861,\"platform\":\"windows\","
        "\"architecture\":\"x86_64\",\"build_configuration\":\"shipping\"},"
        "\"oracle\":{\"executable\":" + file_seal + ",\"binds_cache\":" + binds_oracle +
        ",\"shipping_cache\":" + base_oracle + ",\"depot_manifest\":" + depot_seal +
        ",\"pe_codeview\":{\"guid\":\"guid\",\"age\":1}},"
        "\"binds\":{\"wire_schema_version\":1,\"struct_count\":1,\"class_count\":1,"
        "\"method_count\":1,\"struct_property_count\":1,\"class_property_count\":1,"
        "\"canonical_database_sha256\":\"" + digest + "\"},"
        "\"engine\":{\"as_create_version\":23300,\"ordered_engine_properties\":" + properties_blob +
        ",\"registration_trace\":" + trace_blob + ",\"registration_trace_count\":1,"
        "\"post_bind_snapshot\":" + snapshot_blob + "},"
        "\"unreal_semantics\":{\"reflected_type_graph\":" + common_blob + ",\"metadata_schema_version\":1},"
        "\"frontend\":{\"preprocessor_config\":" + preprocessor_blob +
        ",\"class_generator_config\":" + class_blob + ",\"compiler_options\":" + options_blob + "},"
        "\"bytecode\":{\"opcode_table_version\":\"g1r-v1\",\"opcode_table\":" + common_blob +
        ",\"operand_schema\":" + common_blob + ",\"codegen_probe_corpus\":" + common_blob +
        ",\"expected_probe_results\":" + common_blob + "},"
        "\"cache_writer\":{\"format_version\":1,\"serializer_schema\":" + common_blob +
        ",\"build_identifier\":1,\"reference_table_order\":" + common_blob +
        ",\"normalized_oracle_corpus\":" + common_blob + "},"
        "\"qualification\":{\"required_probe_suite_version\":\"test-v1\","
        "\"diagnostic_parity\":" + common_blob + ",\"semantic_parity\":" + common_blob +
        ",\"qualified\":true}}";
    standalone::sha256 profile_hash;
    constexpr char profile_domain[] = "gore-as-compiler-profile-v1\0";
    profile_hash.update(profile_domain, sizeof(profile_domain) - 1U);
    profile_hash.update(profile_payload);
    const std::string profile_digest = standalone::sha256_hex(profile_hash.finish());
    std::string manifest = profile_payload;
    manifest.pop_back();
    manifest += ",\"profile_sha256\":\"" + profile_digest + "\"}";
    const std::filesystem::path manifest_path = profile_root / "profile.json";
    if (!write_text(manifest_path, manifest)) return 7;

    const std::string request =
        "{\"request_version\":1,\"operation\":\"compile\",\"profile\":{"
        "\"manifest_path\":\"" + json_path(manifest_path) + "\",\"profile_root\":\"" +
        json_path(profile_root) + "\",\"profile_sha256\":\"" + profile_digest +
        "\",\"steam_build_id\":24539464,\"depot_id\":1297901,"
        "\"depot_manifest_gid\":1585071322101748861,\"required_probe_suite_version\":\"test-v1\"},"
        "\"inputs\":{\"base_cache\":" + path_seal(base_path, base) + ",\"binds_cache\":" +
        path_seal(binds_path, binds) + ",\"source_tree\":{\"root\":\"" + json_path(source_root) +
        "\",\"files\":[{\"path\":\"Module.as\",\"byte_len\":" + std::to_string(source_bytes.size()) +
        ",\"sha256\":\"" + standalone::sha256_hex(
            standalone::sha256_bytes(source_bytes.data(), source_bytes.size())) + "\"}]},"
        "\"overlays\":[{\"ordinal\":0,\"operation\":\"add\",\"module_name\":\"Module\","
        "\"relative_path\":\"Module.as\"}]},\"output\":{\"cache_path\":\"" +
        json_path(output_path) + "\"}}";
    const std::filesystem::path request_path = root / "request.json";
    if (!write_text(request_path, request)) return 8;

    const auto result = standalone::compile_sidecar_request(request_path.native());
    if (result.exit_code != standalone::protocol::ExitCode::success ||
        result.response_json.find("\"ok\":true") == std::string::npos) {
        std::cerr << result.response_json;
        std::filesystem::remove_all(root, filesystem_error);
        return 9;
    }
    std::ifstream output_stream(output_path, std::ios::binary);
    const std::vector<std::uint8_t> output_bytes{
        std::istreambuf_iterator<char>(output_stream), std::istreambuf_iterator<char>()};
    precompiled::cache generated;
    if (!precompiled::decode(output_bytes.data(), output_bytes.size(), generated, codec_error) ||
        generated.modules.size() != 1U || generated.modules[0].second.module_name.bytes != "Module" ||
        generated.modules[0].second.functions.size() != 1U ||
        generated.modules[0].second.functions[0].function_name.bytes != "Answer") {
        std::filesystem::remove_all(root, filesystem_error);
        return 10;
    }

    const auto overwrite = standalone::compile_sidecar_request(request_path.native());
    if (overwrite.exit_code == standalone::protocol::ExitCode::success ||
        overwrite.response_json.find("GORE_AS_OUTPUT_CREATE_FAILED") == std::string::npos) {
        std::filesystem::remove_all(root, filesystem_error);
        return 11;
    }
    std::filesystem::remove_all(root, filesystem_error);
    return 0;
}
