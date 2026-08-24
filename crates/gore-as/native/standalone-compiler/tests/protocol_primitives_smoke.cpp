#include "gore_as_standalone/json.hpp"
#include "gore_as_standalone/profile_loader.hpp"
#include "gore_as_standalone/sha256.hpp"

#include <array>
#include <iostream>
#include <string>
#include <string_view>
#include <utility>

namespace {

bool expect(const bool condition, const char* const detail) {
    if (!condition) std::cerr << detail << '\n';
    return condition;
}

std::string fixed_operations(const unsigned size, const unsigned alignment) {
    return "{\"can_create_property\":true,\"never_requires_gc\":false,\"requires_property\":false,"
           "\"can_be_template_subtype\":true,\"can_construct\":true,\"need_construct\":false,"
        "\"can_destruct\":true,\"need_destruct\":false,\"can_copy\":true,\"need_copy\":false,"
        "\"can_compare\":true,\"can_hash_value\":true,\"value_size\":" + std::to_string(size) +
        ",\"value_alignment\":" + std::to_string(alignment) + ",\"is_object_pointer\":false}";
}

std::string sealed_blob(const std::string& path) {
    return "{\"path\":\"" + path + "\",\"byte_len\":12,\"sha256\":\"" +
        std::string(64U, '1') + "\"}";
}

bool profile_loader_smoke() {
    using namespace gore::as::standalone;
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
        "\"result\":{\"kind\":\"enum\",\"engine_type_id\":1}}],\"final_states\":[],"
        "\"canonical_sha256\":\"" + digest + "\"}";
    registry_profile registry;
    std::string detail;
    if (!expect(
            parse_registry_profile_payloads(properties, trace, snapshot, 1U, registry, detail),
            detail.c_str())) return false;
    if (!expect(registry.registrations.size() == 1U && registry.expected_results.size() == 1U,
            "registry JSON projection count mismatch")) return false;

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
        "\"static_class_mode\":\"allowed\",\"script_float_is_float64\":true,"
        "\"angelscript_haze\":false,\"enforce_server_rpc_validation\":false,"
        "\"blueprint_event_argument_specializations\":[\"FName\"],\"native_super_types\":["
        "{\"ordinal\":0,\"angelscript_type_name\":\"UObject\","
        "\"unreal_class_path\":\"/Script/CoreUObject.Object\",\"property_offset\":64,"
        "\"kind\":\"other_u_object\",\"game_state_subsystem\":false,"
        "\"cannot_derive_angelscript\":false}],"
        "\"fname_comparison_keys\":[{\"ordinal\":0,\"spelling\":\"Gr\\u00f6\\u00dfe\","
        "\"comparison_key\":\"fname-key-17\"}],\"external_hooks\":{"
        "\"class_analyze\":{\"bound\":false,\"captures\":[]},"
        "\"process_chunks\":{\"bound\":false,\"captures\":[]},"
        "\"post_process_code\":{\"bound\":false,\"captures\":[]}},"
        "\"canonical_sha256\":\"" + digest + "\"}";
    const std::string class_generator =
        "{\"schema\":\"gore.as.class-generator-config\",\"schema_version\":1,"
        "\"mark_non_uproperty_properties_as_transient\":false,\"canonical_sha256\":\"" + digest + "\"}";
    const std::string compiler_options_json =
        "{\"schema\":\"gore.as.compiler-options\",\"schema_version\":1,"
        "\"error_on_incorrect_editor_only_code\":true,"
        "\"warn_on_divergent_comparison_operator_overloads\":true,"
        "\"warn_on_implicit_signed_unsigned_conversion\":true,"
        "\"warn_on_increment_decrement_in_complex_expression\":true,"
        "\"warn_on_unused_return_value_for_const_methods\":true,\"canonical_sha256\":\"" + digest + "\"}";
    preprocessor_options preprocessor_profile;
    compiler_options compiler_profile;
    external_frontend_profile external_profile;
    if (!expect(parse_frontend_profile_payloads(
            preprocessor, class_generator, compiler_options_json,
            preprocessor_profile, compiler_profile, external_profile, detail), detail.c_str())) return false;
    if (!expect(preprocessor_profile.native_super_types.size() == 1U &&
            preprocessor_profile.fname_comparison_keys.size() == 1U &&
            !external_profile.class_analyze_bound &&
            compiler_profile.error_on_incorrect_editor_only_code,
            "frontend JSON projection mismatch")) return false;

    const std::string generated_statics = "void CapturedStatic() {}";
    std::string captured_preprocessor = preprocessor;
    const std::string unbound_class =
        "\"class_analyze\":{\"bound\":false,\"captures\":[]}";
    const std::string captured_class =
        "\"class_analyze\":{\"bound\":true,\"captures\":[{\"ordinal\":0,"
        "\"module_name\":\"Game.Module\",\"namespace\":\"Game\","
        "\"class_name\":\"ACaptured\",\"source_sha256\":\"" + digest +
        "\",\"input_generated_statics_sha256\":\"" + digest +
        "\",\"generated_statics\":\"" + generated_statics +
        "\",\"output_generated_statics_sha256\":\"" +
        sha256_hex(sha256_bytes(generated_statics.data(), generated_statics.size())) +
        "\",\"has_statics\":true,\"compose_onto_class\":\"AParent\"}]}";
    const std::size_t class_position = captured_preprocessor.find(unbound_class);
    if (!expect(class_position != std::string::npos, "unbound hook fixture is missing")) return false;
    captured_preprocessor.replace(class_position, unbound_class.size(), captured_class);
    if (!expect(parse_frontend_profile_payloads(
            captured_preprocessor, class_generator, compiler_options_json,
            preprocessor_profile, compiler_profile, external_profile, detail), detail.c_str()) ||
        !expect(external_profile.class_analyze_bound &&
            external_profile.class_analyze_captures.size() == 1U &&
            external_profile.class_analyze_captures[0].compose_onto_class == "AParent",
            "captured ClassAnalyze projection mismatch")) return false;

    std::string missing_capture_contract = preprocessor;
    const std::size_t fname_begin = missing_capture_contract.find(",\"fname_comparison_keys\"");
    const std::size_t hook_begin = missing_capture_contract.find(",\"external_hooks\"");
    if (!expect(fname_begin != std::string::npos && hook_begin > fname_begin,
            "capture-contract fixture boundaries are missing")) return false;
    missing_capture_contract.erase(fname_begin, hook_begin - fname_begin);
    if (!expect(!parse_frontend_profile_payloads(
            missing_capture_contract, class_generator, compiler_options_json,
            preprocessor_profile, compiler_profile, external_profile, detail),
            "missing FName capture contract was accepted")) return false;

    const std::string file_seal =
        "{\"byte_len\":3,\"sha256\":\"" + digest + "\",\"steam_content_sha1\":\"" +
        std::string(40U, '2') + "\"}";
    const std::string depot_seal = "{\"byte_len\":3,\"sha256\":\"" + digest + "\"}";
    const std::string blob = sealed_blob("blob.bin");
    const std::string payload =
        "{\"schema\":\"gore.as.compiler-profile\",\"schema_version\":1,"
        "\"target\":{\"steam_app_id\":1297900,\"steam_build_id\":24539464,\"depot_id\":1297901,"
        "\"depot_manifest_gid\":1585071322101748861,\"platform\":\"windows\","
        "\"architecture\":\"x86_64\",\"build_configuration\":\"shipping\"},"
        "\"oracle\":{\"executable\":" + file_seal + ",\"binds_cache\":" + file_seal +
        ",\"shipping_cache\":" + file_seal + ",\"depot_manifest\":" + depot_seal +
        ",\"pe_codeview\":{\"guid\":\"guid\",\"age\":1}},"
        "\"binds\":{\"wire_schema_version\":1,\"struct_count\":1,\"class_count\":1,"
        "\"method_count\":1,\"struct_property_count\":1,\"class_property_count\":1,"
        "\"canonical_database_sha256\":\"" + digest + "\"},"
        "\"engine\":{\"as_create_version\":23300,\"ordered_engine_properties\":" + blob +
        ",\"registration_trace\":" + blob + ",\"registration_trace_count\":1,"
        "\"post_bind_snapshot\":" + blob + "},"
        "\"unreal_semantics\":{\"reflected_type_graph\":" + blob + ",\"metadata_schema_version\":1},"
        "\"frontend\":{\"preprocessor_config\":" + blob + ",\"class_generator_config\":" + blob +
        ",\"compiler_options\":" + blob + "},"
        "\"bytecode\":{\"opcode_table_version\":\"g1r-v1\",\"opcode_table\":" + blob +
        ",\"operand_schema\":" + blob + ",\"codegen_probe_corpus\":" + blob +
        ",\"expected_probe_results\":" + blob + "},"
        "\"cache_writer\":{\"format_version\":1,\"serializer_schema\":" + blob +
        ",\"build_identifier\":2654436030,\"reference_table_order\":" + blob +
        ",\"normalized_oracle_corpus\":" + blob + "},"
        "\"qualification\":{\"required_probe_suite_version\":\"test-v1\",\"diagnostic_parity\":" + blob +
        ",\"semantic_parity\":" + blob + ",\"qualified\":true}}";
    sha256 profile_hash;
    constexpr char domain[] = "gore-as-compiler-profile-v1\0";
    profile_hash.update(domain, sizeof(domain) - 1U);
    profile_hash.update(payload);
    std::string manifest_json = payload;
    manifest_json.pop_back();
    manifest_json += ",\"profile_sha256\":\"" + sha256_hex(profile_hash.finish()) + "\"}";
    compiler_profile_manifest manifest;
    if (!expect(parse_compiler_profile_manifest(manifest_json, manifest, detail), detail.c_str())) return false;
    if (!expect(manifest.build_identifier < 0 && manifest.all_blobs.size() == 16U,
            "compiler manifest projection mismatch")) return false;
    for (const auto& drift : std::array<std::pair<std::string_view, std::string_view>, 3U>{{
             {"\"steam_app_id\":1297900", "\"steam_app_id\":1297901"},
             {"\"steam_build_id\":24539464", "\"steam_build_id\":24539465"},
             {"\"depot_id\":1297901", "\"depot_id\":1297902"},
         }}) {
        std::string foreign_payload = payload;
        const std::size_t position = foreign_payload.find(drift.first);
        if (!expect(position != std::string::npos, "target drift fixture field is missing")) {
            return false;
        }
        foreign_payload.replace(position, drift.first.size(), drift.second);
        sha256 foreign_hash;
        foreign_hash.update(domain, sizeof(domain) - 1U);
        foreign_hash.update(foreign_payload);
        std::string foreign_manifest = foreign_payload;
        foreign_manifest.pop_back();
        foreign_manifest += ",\"profile_sha256\":\"" +
            sha256_hex(foreign_hash.finish()) + "\"}";
        compiler_profile_manifest rejected;
        if (!expect(
                !parse_compiler_profile_manifest(foreign_manifest, rejected, detail),
                "resealed foreign compiler target was accepted")) {
            return false;
        }
    }
    return true;
}

} // namespace

int main() {
    using namespace gore::as::standalone;

    const auto empty = sha256_bytes(nullptr, 0U);
    if (!expect(
            sha256_hex(empty) == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "empty SHA-256 vector mismatch")) return 1;
    const std::string abc = "abc";
    if (!expect(
            sha256_hex(sha256_bytes(abc.data(), abc.size())) ==
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "abc SHA-256 vector mismatch")) return 1;

    json::value root;
    json::parse_error error;
    const std::string document =
        R"({"request_version":1,"text":"A\u00df\ud834\udd1e","ok":true,"none":null,"items":[0,-1,18446744073709551615]})";
    if (!expect(json::parse(document, 8U, root, error), error.detail.c_str())) return 1;
    std::string detail;
    if (!expect(
            json::require_object_keys(
                root,
                {"request_version", "text", "ok", "none", "items"},
                {}, detail),
            detail.c_str())) return 1;
    std::string text;
    if (!expect(json::get_string(root, "text", text, detail), detail.c_str())) return 1;
    if (!expect(text == "A\xc3\x9f\xf0\x9d\x84\x9e", "unicode escape decoding mismatch")) return 1;
    std::string compact;
    if (!expect(json::serialize_compact(root, compact), "compact JSON serialization failed")) return 1;
    const std::string expected_compact =
        std::string("{\"request_version\":1,\"text\":\"") + text +
        "\",\"ok\":true,\"none\":null,\"items\":[0,-1,18446744073709551615]}";
    if (!expect(compact == expected_compact, "compact JSON serialization mismatch")) return 1;

    for (const std::string invalid : {
             R"({"a":1,"a":2})",
             R"({"a":1.0})",
             R"({"a":"\ud800"})",
             R"({"a":01})",
         }) {
        json::value rejected;
        json::parse_error rejected_error;
        if (!expect(!json::parse(invalid, 8U, rejected, rejected_error), "invalid JSON was accepted")) {
            return 1;
        }
    }

    if (!profile_loader_smoke()) return 1;

    return 0;
}
