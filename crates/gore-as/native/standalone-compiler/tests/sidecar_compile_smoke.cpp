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
    return "{\"can_create_property\":true,\"never_requires_gc\":false,\"requires_property\":false,"
           "\"can_be_template_subtype\":true,\"can_construct\":true,\"need_construct\":false,"
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

std::string source_seal_fields(const std::vector<std::uint8_t>& bytes) {
    return "\"source_byte_len\":" + std::to_string(bytes.size()) +
        ",\"source_sha256\":\"" + standalone::sha256_hex(
            standalone::sha256_bytes(bytes.data(), bytes.size())) + "\"";
}

std::string source_file(const std::string& path, const std::vector<std::uint8_t>& bytes) {
    return "{\"path\":\"" + path + "\",\"byte_len\":" +
        std::to_string(bytes.size()) + ",\"sha256\":\"" + standalone::sha256_hex(
            standalone::sha256_bytes(bytes.data(), bytes.size())) + "\"}";
}

bool replace_once(std::string& text, const std::string& from, const std::string& to) {
    const auto position = text.find(from);
    if (position == std::string::npos) return false;
    text.replace(position, from.size(), to);
    return true;
}

precompiled::map_string module_key(const std::string& name) {
    precompiled::map_string key;
    key.payload.assign(name.begin(), name.end());
    return key;
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
    std::filesystem::create_directories(source_root / "Editor", filesystem_error);
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
        "\"automatic_imports\":false,\"warn_on_manual_import_statements\":true,"
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
        "\"fname_comparison_keys\":[],\"external_hooks\":{"
        "\"class_analyze\":{\"bound\":false,\"captures\":[]},"
        "\"process_chunks\":{\"bound\":false,\"captures\":[]},"
        "\"post_process_code\":{\"bound\":false,\"captures\":[]}},"
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
    for (const char* const name : {"A", "B"}) {
        precompiled::precompiled_module module;
        module.module_name.bytes = name;
        module.script_relative_filename.bytes = std::string(name) + ".as";
        empty_cache.modules.emplace_back(module_key(name), std::move(module));
    }
    std::vector<std::uint8_t> base;
    precompiled::codec_error codec_error;
    if (!precompiled::encode(empty_cache, base, codec_error)) return 5;
    const std::vector<std::uint8_t> binds{'b', 'i', 'n', 'd', 's'};
    empty_cache.data_guid[0] = 0x5aU;
    std::vector<std::uint8_t> compatible_base;
    if (!precompiled::encode(empty_cache, compatible_base, codec_error)) return 72;
    const std::vector<std::uint8_t> compatible_binds{
        'b', 0U, 'i', 0U, 'n', 0U, 'd', 0U, 's', 0U};
    const std::string dependency_source =
        "struct FAnswerInput { int Value; }\n";
    const std::string source =
        "import Dependency;\nint Answer(FAnswerInput Input) { return Input.Value + 2; }\n";
    const std::string editor_source = "this is intentionally not valid script\n";
    const std::vector<std::uint8_t> dependency_bytes(
        dependency_source.begin(), dependency_source.end());
    const std::vector<std::uint8_t> source_bytes(source.begin(), source.end());
    const std::vector<std::uint8_t> editor_bytes(editor_source.begin(), editor_source.end());
    const std::string edit_a_source = "int EditedA() { return 41; }\n";
    const std::string add_c_source =
        "int QualificationPrimitive() { return 42; }\n"
        "int AddedC() { return QualificationPrimitive(); }\n";
    const std::vector<std::uint8_t> edit_a_bytes(edit_a_source.begin(), edit_a_source.end());
    const std::vector<std::uint8_t> add_c_bytes(add_c_source.begin(), add_c_source.end());
    std::string add_c_json = add_c_source;
    for (std::size_t position = 0U; (position = add_c_json.find('\n', position)) != std::string::npos;) {
        add_c_json.replace(position, 1U, "\\n");
        position += 2U;
    }
    const std::string add_c_sha = standalone::sha256_hex(
        standalone::sha256_bytes(add_c_bytes.data(), add_c_bytes.size()));
    const auto corpus_case = [&](const std::uint32_t ordinal, const std::string& case_id,
                                 const std::string& declaration) {
        return "{\"ordinal\":" + std::to_string(ordinal) + ",\"case_id\":\"" + case_id +
            "\",\"category\":\"invoke\",\"expected_outcome\":\"accepted\"," +
            "\"mode\":{\"kind\":\"invoke\",\"declaration\":\"" + declaration +
            "\"},\"sections\":[{\"ordinal\":0,\"module\":\"C\"," +
            "\"relative_path\":\"C.as\",\"source_utf8\":\"" + add_c_json +
            "\",\"source_sha256\":\"" + add_c_sha + "\"}]}";
    };
    const std::string qualification_corpus =
        std::string("{\"schema\":\"gore.as.compiler-probe-corpus\",\"schema_version\":2,") +
        "\"suite_id\":\"test-v1\",\"cases\":[" +
        corpus_case(0U, "positive.primitive", "int QualificationPrimitive()") + "," +
        corpus_case(1U, "negative.unsafe-call", "int AddedC()") +
        "],\"canonical_sha256\":\"" + std::string(64U, '1') + "\"}";
    const std::filesystem::path base_path = root / "base.cache";
    const std::filesystem::path binds_path = root / "binds.cache";
    const std::filesystem::path compatible_base_path = root / "compatible-base.cache";
    const std::filesystem::path compatible_binds_path = root / "compatible-binds.cache";
    const std::filesystem::path source_path = source_root / "Module.as";
    const std::filesystem::path dependency_path = source_root / "Dependency.as";
    const std::filesystem::path editor_path = source_root / "Editor" / "Ignored.as";
    const std::filesystem::path output_path = output_root / "generated.cache";
    const std::filesystem::path edit_a_path = source_root / "A.as";
    const std::filesystem::path add_c_path = source_root / "C.as";
    const std::filesystem::path full_graph_output_path = output_root / "full-graph.cache";
    if (!write_bytes(base_path, base) || !write_bytes(binds_path, binds) ||
        !write_bytes(compatible_base_path, compatible_base) ||
        !write_bytes(compatible_binds_path, compatible_binds) ||
        !write_bytes(dependency_path, dependency_bytes) ||
        !write_bytes(editor_path, editor_bytes) ||
        !write_bytes(source_path, source_bytes) ||
        !write_bytes(edit_a_path, edit_a_bytes) ||
        !write_bytes(add_c_path, add_c_bytes) ||
        !write_text(profile_root / "qualification-corpus.json", qualification_corpus)) return 6;

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
    const std::string corpus_blob = blob("qualification-corpus.json", qualification_corpus);
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
        ",\"operand_schema\":" + common_blob + ",\"codegen_probe_corpus\":" + corpus_blob +
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
    std::string unqualified_payload = profile_payload;
    const auto qualified_field = unqualified_payload.find("\"qualified\":true");
    if (qualified_field == std::string::npos) return 70;
    unqualified_payload.replace(
        qualified_field, std::string("\"qualified\":true").size(), "\"qualified\":false");
    standalone::sha256 unqualified_hash;
    unqualified_hash.update(profile_domain, sizeof(profile_domain) - 1U);
    unqualified_hash.update(unqualified_payload);
    const std::string unqualified_digest =
        standalone::sha256_hex(unqualified_hash.finish());
    std::string unqualified_manifest = unqualified_payload;
    unqualified_manifest.pop_back();
    unqualified_manifest += ",\"profile_sha256\":\"" + unqualified_digest + "\"}";
    const std::filesystem::path unqualified_manifest_path =
        profile_root / "profile-unqualified.json";
    if (!write_text(unqualified_manifest_path, unqualified_manifest)) return 71;

    const std::string request =
        "{\"request_version\":1,\"operation\":\"compile\",\"profile\":{"
        "\"manifest_path\":\"" + json_path(manifest_path) + "\",\"profile_root\":\"" +
        json_path(profile_root) + "\",\"profile_sha256\":\"" + profile_digest +
        "\",\"steam_build_id\":24539464,\"depot_id\":1297901,"
        "\"depot_manifest_gid\":1585071322101748861,\"required_probe_suite_version\":\"test-v1\"},"
        "\"inputs\":{\"base_cache\":" + path_seal(base_path, base) + ",\"binds_cache\":" +
        path_seal(binds_path, binds) + ",\"source_tree\":{\"root\":\"" + json_path(source_root) +
        "\",\"files\":[{\"path\":\"Dependency.as\",\"byte_len\":" +
        std::to_string(dependency_bytes.size()) + ",\"sha256\":\"" +
        standalone::sha256_hex(standalone::sha256_bytes(
            dependency_bytes.data(), dependency_bytes.size())) + "\"},"
        "{\"path\":\"Editor/Ignored.as\",\"byte_len\":" +
        std::to_string(editor_bytes.size()) + ",\"sha256\":\"" +
        standalone::sha256_hex(standalone::sha256_bytes(
            editor_bytes.data(), editor_bytes.size())) + "\"},"
        "{\"path\":\"Module.as\",\"byte_len\":" + std::to_string(source_bytes.size()) +
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
        generated.modules.size() != 4U ||
        generated.modules[0].second.module_name.bytes != "A" ||
        generated.modules[1].second.module_name.bytes != "B" ||
        generated.modules[2].second.module_name.bytes != "Dependency" ||
        generated.modules[3].second.module_name.bytes != "Module" ||
        generated.modules[3].second.functions.size() != 1U ||
        generated.modules[3].second.functions[0].function_name.bytes != "Answer") {
        std::filesystem::remove_all(root, filesystem_error);
        return 10;
    }

    const auto overwrite = standalone::compile_sidecar_request(request_path.native());
    if (overwrite.exit_code == standalone::protocol::ExitCode::success ||
        overwrite.response_json.find("GORE_AS_OUTPUT_CREATE_FAILED") == std::string::npos) {
        std::filesystem::remove_all(root, filesystem_error);
        return 11;
    }

    // Product execution trusts the parent resolver's semantic compatibility decision and still
    // seals the exact staged bytes. A different cache GUID and a different Binds representation
    // must therefore reach the compiler instead of being re-bound to the qualification oracle.
    const std::filesystem::path compatible_output_path = output_root / "compatible.cache";
    std::string compatible_request = request;
    if (!replace_once(compatible_request, path_seal(base_path, base),
            path_seal(compatible_base_path, compatible_base)) ||
        !replace_once(compatible_request, path_seal(binds_path, binds),
            path_seal(compatible_binds_path, compatible_binds)) ||
        !replace_once(compatible_request, json_path(output_path),
            json_path(compatible_output_path))) return 73;
    const std::filesystem::path compatible_request_path = root / "compatible-request.json";
    if (!write_text(compatible_request_path, compatible_request)) return 74;
    const auto compatible_result =
        standalone::compile_sidecar_request(compatible_request_path.native());
    if (compatible_result.exit_code != standalone::protocol::ExitCode::success ||
        compatible_result.response_json.find("\"ok\":true") == std::string::npos) {
        std::cerr << compatible_result.response_json;
        std::filesystem::remove_all(root, filesystem_error);
        return 75;
    }


    const std::string request_prefix_v2 =
        "{\"request_version\":2,\"operation\":\"compile\",\"profile\":{"
        "\"manifest_path\":\"" + json_path(manifest_path) + "\",\"profile_root\":\"" +
        json_path(profile_root) + "\",\"profile_sha256\":\"" + profile_digest +
        "\",\"steam_build_id\":24539464,\"depot_id\":1297901,"
        "\"depot_manifest_gid\":1585071322101748861,\"required_probe_suite_version\":\"test-v1\"},"
        "\"inputs\":{\"base_cache\":" + path_seal(base_path, base) + ",\"binds_cache\":" +
        path_seal(binds_path, binds) + ",\"source_tree\":{\"root\":\"" + json_path(source_root) +
        "\",\"files\":[";
    const std::string source_files_v2 =
        source_file("A.as", edit_a_bytes) + "," + source_file("C.as", add_c_bytes);
    const std::string changes_v2 =
        "]},\"changes\":[{\"ordinal\":0,\"operation\":\"edit\",\"module_name\":\"A\","
        "\"relative_path\":\"A.as\"," + source_seal_fields(edit_a_bytes) + "},{"
        "\"ordinal\":1,\"operation\":\"delete\",\"module_name\":\"B\","
        "\"relative_path\":\"B.as\"},{\"ordinal\":2,\"operation\":\"add\","
        "\"module_name\":\"C\",\"relative_path\":\"C.as\"," +
        source_seal_fields(add_c_bytes) + "}],\"final_manifest\":[";
    const std::string final_manifest_v2 =
        "{\"ordinal\":0,\"module_name\":\"A\",\"relative_path\":\"A.as\"},"
        "{\"ordinal\":1,\"module_name\":\"C\",\"relative_path\":\"C.as\"}";
    const auto full_graph_request = [&](const std::string& files,
                                        const std::string& changes,
                                        const std::string& final_manifest,
                                        const std::filesystem::path& output) {
        return request_prefix_v2 + files + changes + final_manifest +
            "]},\"output\":{\"cache_path\":\"" + json_path(output) + "\"}}";
    };
    const std::string request_v2 = full_graph_request(
        source_files_v2, changes_v2, final_manifest_v2, full_graph_output_path);
    const std::filesystem::path request_path_v2 = root / "request-v2.json";
    if (!write_text(request_path_v2, request_v2)) return 12;
    const auto full_graph_result = standalone::compile_sidecar_request(request_path_v2.native());
    if (full_graph_result.exit_code != standalone::protocol::ExitCode::success ||
        full_graph_result.response_json.find("\"ok\":true") == std::string::npos) {
        std::cerr << full_graph_result.response_json;
        std::filesystem::remove_all(root, filesystem_error);
        return 13;
    }
    std::ifstream full_graph_stream(full_graph_output_path, std::ios::binary);
    const std::vector<std::uint8_t> full_graph_bytes{
        std::istreambuf_iterator<char>(full_graph_stream), std::istreambuf_iterator<char>()};
    precompiled::cache full_graph;
    if (!precompiled::decode(
            full_graph_bytes.data(), full_graph_bytes.size(), full_graph, codec_error) ||
        full_graph.modules.size() != 2U ||
        full_graph.modules[0].second.module_name.bytes != "A" ||
        full_graph.modules[0].second.script_relative_filename.bytes != "A.as" ||
        full_graph.modules[0].second.functions.size() != 1U ||
        full_graph.modules[0].second.functions[0].function_name.bytes != "EditedA" ||
        full_graph.modules[1].second.module_name.bytes != "C" ||
        full_graph.modules[1].second.script_relative_filename.bytes != "C.as" ||
        full_graph.modules[1].second.functions.size() != 2U ||
        std::all_of(full_graph.data_guid.begin(), full_graph.data_guid.end(),
            [](const std::uint8_t byte) { return byte == 0U; })) {
        std::filesystem::remove_all(root, filesystem_error);
        return 14;
    }

    // Qualification v3 is a separate command capability: the product compile entry point must
    // reject it, while the explicit qualification entry point returns same-run evidence. No
    // caller-provided witness member is accepted.
    const auto qualification_request = [&](const std::filesystem::path& output,
                                           const std::string& invoke_declaration,
                                           const bool inject_witness) {
        const std::string qualification_changes =
            std::string("]},\"changes\":[{\"ordinal\":0,\"operation\":\"add\",\"module_name\":\"C\",") +
            "\"relative_path\":\"C.as\"," + source_seal_fields(add_c_bytes) +
            "}],\"final_manifest\":[";
        const std::string qualification_final =
            "{\"ordinal\":0,\"module_name\":\"A\",\"relative_path\":\"A.as\"},"
            "{\"ordinal\":1,\"module_name\":\"B\",\"relative_path\":\"B.as\"},"
            "{\"ordinal\":2,\"module_name\":\"C\",\"relative_path\":\"C.as\"}";
        std::string body = full_graph_request(
            source_file("C.as", add_c_bytes), qualification_changes, qualification_final, output);
        const auto manifest_identity = body.find(json_path(manifest_path));
        const auto profile_identity = body.find(profile_digest);
        if (manifest_identity == std::string::npos || profile_identity == std::string::npos) {
            return std::string{};
        }
        body.replace(manifest_identity, json_path(manifest_path).size(),
            json_path(unqualified_manifest_path));
        // Re-find after the manifest path replacement, which can change string offsets.
        const auto current_profile_identity = body.find(profile_digest);
        if (current_profile_identity == std::string::npos) return std::string{};
        body.replace(current_profile_identity, profile_digest.size(), unqualified_digest);
        const std::string compile_prefix = "{\"request_version\":2,\"operation\":\"compile\"";
        const auto prefix = body.find(compile_prefix);
        if (prefix == std::string::npos || body.empty() || body.back() != '}') return std::string{};
        body.replace(prefix, compile_prefix.size(),
            "{\"request_version\":3,\"operation\":\"qualify\"");
        body.pop_back();
        body += ",\"qualification\":{\"suite_id\":\"test-v1\",";
        body += "\"corpus_sha256\":\"" + std::string(64U, '1') + "\",";
        const std::string case_id = invoke_declaration == "int AddedC()"
            ? "negative.unsafe-call" : "positive.primitive";
        body += "\"case_id\":\"" + case_id + "\",\"phase\":\"single\",";
        body += "\"invoke_module\":\"" +
            std::string(invoke_declaration.empty() ? "" : "C") +
            "\",\"invoke_declaration\":\"" + invoke_declaration + "\"";
        if (inject_witness) body += ",\"frontend_witness\":{}";
        body += "}}";
        return body;
    };
    std::string incompatible_qualification = qualification_request(
        output_root / "qualification-compatible-target.cache", "", false);
    if (!replace_once(incompatible_qualification, path_seal(base_path, base),
            path_seal(compatible_base_path, compatible_base)) ||
        !replace_once(incompatible_qualification, path_seal(binds_path, binds),
            path_seal(compatible_binds_path, compatible_binds))) return 76;
    const auto incompatible_qualification_path = root / "qualification-compatible-target.json";
    if (!write_text(incompatible_qualification_path, incompatible_qualification)) return 77;
    const auto incompatible_qualification_result = standalone::compile_sidecar_request(
        incompatible_qualification_path.native(), true);
    if (incompatible_qualification_result.exit_code ==
            standalone::protocol::ExitCode::success ||
        incompatible_qualification_result.response_json.find("GORE_AS_ORACLE_INPUT_MISMATCH") ==
            std::string::npos) {
        std::filesystem::remove_all(root, filesystem_error);
        return 78;
    }
    const auto qualification_path = root / "qualification-v3.json";
    if (!write_text(qualification_path,
            qualification_request(
                output_root / "qualification.cache",
                "int QualificationPrimitive()", false))) return 90;
    const auto product_rejects_qualification =
        standalone::compile_sidecar_request(qualification_path.native());
    if (product_rejects_qualification.exit_code == standalone::protocol::ExitCode::success) {
        std::filesystem::remove_all(root, filesystem_error);
        return 91;
    }
    const auto qualification =
        standalone::compile_sidecar_request(qualification_path.native(), true);
    if (qualification.exit_code != standalone::protocol::ExitCode::success ||
        qualification.response_json.find("\"protocol_version\":3") == std::string::npos ||
        qualification.response_json.find("\"kind\":\"i64\",\"value\":42") ==
            std::string::npos ||
        qualification.response_json.find(
            "\"resolve_object_ptr_callback_registered\":false") == std::string::npos ||
        qualification.response_json.find("\"caller_witnesses\"") != std::string::npos) {
        std::cerr << qualification.response_json;
        std::filesystem::remove_all(root, filesystem_error);
        return 92;
    }
    const auto witness_path = root / "qualification-witness-injection.json";
    if (!write_text(witness_path,
            qualification_request(
                output_root / "qualification-witness.cache",
                "int QualificationPrimitive()", true))) return 93;
    const auto witness_rejected =
        standalone::compile_sidecar_request(witness_path.native(), true);
    if (witness_rejected.exit_code == standalone::protocol::ExitCode::success) {
        std::filesystem::remove_all(root, filesystem_error);
        return 94;
    }
    const auto unsafe_invoke_path = root / "qualification-unsafe-invoke.json";
    if (!write_text(unsafe_invoke_path,
            qualification_request(
                output_root / "qualification-unsafe.cache", "int AddedC()", false))) return 95;
    const auto unsafe_invoke =
        standalone::compile_sidecar_request(unsafe_invoke_path.native(), true);
    if (unsafe_invoke.exit_code == standalone::protocol::ExitCode::success ||
        unsafe_invoke.response_json.find("GORE_AS_QUALIFICATION_INVOKE_UNAVAILABLE") ==
            std::string::npos) {
        std::filesystem::remove_all(root, filesystem_error);
        return 96;
    }

    const std::filesystem::path delete_only_output = output_root / "delete-only.cache";
    const std::string delete_only_changes =
        "]},\"changes\":[{\"ordinal\":0,\"operation\":\"delete\","
        "\"module_name\":\"B\",\"relative_path\":\"B.as\"}],"
        "\"final_manifest\":[";
    const std::string delete_only_final =
        "{\"ordinal\":0,\"module_name\":\"A\",\"relative_path\":\"A.as\"}";
    const std::filesystem::path delete_only_request_path = root / "delete-only.json";
    if (!write_text(
            delete_only_request_path,
            full_graph_request("", delete_only_changes, delete_only_final, delete_only_output))) {
        return 15;
    }
    const auto delete_only_result =
        standalone::compile_sidecar_request(delete_only_request_path.native());
    if (delete_only_result.exit_code != standalone::protocol::ExitCode::success) {
        std::cerr << delete_only_result.response_json;
        std::filesystem::remove_all(root, filesystem_error);
        return 16;
    }
    std::ifstream delete_only_stream(delete_only_output, std::ios::binary);
    const std::vector<std::uint8_t> delete_only_bytes{
        std::istreambuf_iterator<char>(delete_only_stream),
        std::istreambuf_iterator<char>()};
    precompiled::cache delete_only;
    if (!precompiled::decode(
            delete_only_bytes.data(), delete_only_bytes.size(), delete_only, codec_error) ||
        delete_only.modules.size() != 1U ||
        delete_only.modules[0].second.module_name.bytes != "A" ||
        delete_only.modules[0].second.script_relative_filename.bytes != "A.as" ||
        delete_only.data_guid == empty_cache.data_guid ||
        delete_only.data_guid == full_graph.data_guid) {
        std::filesystem::remove_all(root, filesystem_error);
        return 17;
    }

    const auto rejected = [&](const std::string& body, const char* const name) {
        const std::filesystem::path path = root / (std::string(name) + ".json");
        if (!write_text(path, body)) return false;
        const auto result = standalone::compile_sidecar_request(path.native());
        return result.exit_code != standalone::protocol::ExitCode::success &&
            result.response_json.find("\"ok\":false") != std::string::npos;
    };
    const std::string mismatched_final =
        final_manifest_v2 + ",{\"ordinal\":2,\"module_name\":\"D\",\"relative_path\":\"D.as\"}";
    if (!rejected(full_graph_request(
            source_files_v2, changes_v2, mismatched_final,
            output_root / "bad-final.cache"), "bad-final")) {
        std::filesystem::remove_all(root, filesystem_error);
        return 18;
    }
    if (!rejected(full_graph_request(
            source_files_v2 + "," + source_file("Module.as", source_bytes),
            changes_v2, final_manifest_v2, output_root / "undeclared.cache"),
            "undeclared-source")) {
        std::filesystem::remove_all(root, filesystem_error);
        return 19;
    }
    const std::string delete_with_source =
        "]},\"changes\":[{\"ordinal\":0,\"operation\":\"edit\",\"module_name\":\"A\","
        "\"relative_path\":\"A.as\"," + source_seal_fields(edit_a_bytes) + "},{"
        "\"ordinal\":1,\"operation\":\"delete\",\"module_name\":\"B\","
        "\"relative_path\":\"B.as\"," + source_seal_fields(source_bytes) + "},{"
        "\"ordinal\":2,\"operation\":\"add\",\"module_name\":\"C\","
        "\"relative_path\":\"C.as\"," + source_seal_fields(add_c_bytes) +
        "}],\"final_manifest\":[";
    if (!rejected(full_graph_request(
            source_files_v2 + "," + source_file("B.as", source_bytes),
            delete_with_source, final_manifest_v2, output_root / "delete-source.cache"),
            "delete-source")) {
        std::filesystem::remove_all(root, filesystem_error);
        return 20;
    }
    std::string add_collision = changes_v2;
    const std::size_t edit_operation = add_collision.find("\"operation\":\"edit\"");
    if (edit_operation == std::string::npos) return 21;
    add_collision.replace(edit_operation, std::string("\"operation\":\"edit\"").size(),
        "\"operation\":\"add\"");
    if (!rejected(full_graph_request(
            source_files_v2, add_collision, final_manifest_v2,
            output_root / "add-collision.cache"), "add-collision")) {
        std::filesystem::remove_all(root, filesystem_error);
        return 22;
    }
    std::filesystem::remove_all(root, filesystem_error);
    return 0;
}
