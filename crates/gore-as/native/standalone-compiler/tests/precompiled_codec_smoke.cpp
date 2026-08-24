#include "gore_as_standalone/precompiled_data.hpp"

#include <cstdint>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

namespace precompiled = gore::as::standalone::precompiled;

namespace {

precompiled::archive_string sia(const char* const value) {
    return precompiled::archive_string{std::string(value)};
}

precompiled::map_string ansi_key(const char* const value) {
    const std::string text(value);
    return precompiled::map_string{
        false, std::vector<std::uint8_t>(text.begin(), text.end())};
}

precompiled::data_type integer_type() {
    precompiled::data_type type;
    type.token_type = 0x44;
    return type;
}

precompiled::precompiled_function full_function(const char* const name) {
    precompiled::precompiled_function function;
    function.function_name = sia(name);
    function.name_space = sia("Fixture");
    function.return_type = integer_type();
    function.parameter_types = {integer_type()};
    function.parameter_names = {sia("Value")};
    function.parameter_flags = {2};
    function.parameter_default_args = {sia("7")};
    function.function_traits = 0x40004;
    function.byte_code = {1, -2, 3};
    function.byte_code_references = {1};
    function.variable_space = 3;
    function.object_variable_types = {0x1122334455667788LL};
    function.object_variable_positions = {-2};
    function.object_variables_on_heap = 1;
    function.variable_info_program_positions = {2};
    function.variable_info_offsets = {-2};
    function.variable_info_options = {1};
    function.stack_needed = 4;
    function.id = 0x76543210U;
    function.declared_at = 9;
    function.line_numbers = {10, 20};
    function.is_unreal_function = true;
    function.unreal_function_name = sia("FixtureFunction");
    function.metadata_specifiers = {sia("Category")};
    function.metadata_values = {sia("Codec")};
    function.blueprint_callable = true;
    function.blueprint_override = true;
    function.blueprint_event = true;
    function.blueprint_pure = true;
    function.net_function = true;
    function.net_multicast = true;
    function.net_client = true;
    function.net_server = true;
    function.net_validate = true;
    function.unreliable = true;
    function.blueprint_authority_only = true;
    function.exec = true;
    function.can_override_event = true;
    function.dev_function = true;
    function.is_static = true;
    function.is_const_method = true;
    function.thread_safe = true;
    function.is_no_op = true;
    return function;
}

precompiled::cache full_fixture() {
    precompiled::cache fixture;
    for (std::size_t index = 0U; index < fixture.data_guid.size(); ++index) {
        fixture.data_guid[index] = static_cast<std::uint8_t>(index);
    }
    fixture.build_identifier = static_cast<std::int32_t>(0x9e377abeU);

    precompiled::precompiled_property property;
    property.name = sia("Health");
    property.type = integer_type();
    property.is_private = true;
    property.is_protected = true;
    property.is_unreal_property = true;
    property.metadata_specifiers = {sia("ClampMin")};
    property.metadata_values = {sia("0")};
    property.blueprint_readable = true;
    property.blueprint_writable = true;
    property.edit_const = true;
    property.editable_on_defaults = true;
    property.editable_on_instance = true;
    property.instanced_reference = true;
    property.persistent_instance = true;
    property.advanced_display = true;
    property.transient = true;
    property.replicated = true;
    property.skip_replication = true;
    property.skip_serialization = true;
    property.save_game = true;
    property.replication_condition = 6;
    property.rep_notify = true;
    property.config = true;
    property.interp = true;
    property.asset_registry_searchable = true;

    precompiled::precompiled_class script_class;
    script_class.class_name = sia("UCodecFixture");
    script_class.name_space = sia("Fixture");
    script_class.flags = 0x1234;
    script_class.properties = {property};
    script_class.methods = {full_function("Method")};
    script_class.method_table = {0, 1};
    script_class.derived_from = 0x1000;
    script_class.shadow_type = 0x2000;
    script_class.constructors = {full_function("Ctor")};
    script_class.factory_references = {0x3000};
    script_class.behaviour_references = {0x4000};
    script_class.behaviour_functions = {full_function("Behaviour")};
    script_class.behaviour_function_types = {1};
    script_class.is_in_preprocessor = true;
    script_class.super_class = sia("UObject");
    script_class.code_super_class = sia("/Script/CoreUObject.Object");
    script_class.super_is_code_class = true;
    script_class.abstract = true;
    script_class.transient = true;
    script_class.hide_dropdown = true;
    script_class.default_to_instanced = true;
    script_class.edit_inline_new = true;
    script_class.is_deprecated_class = true;
    // Non-empty ConfigName specifically guards the variable-width field that
    // the former skip-only Rust walk could not model losslessly.
    script_class.config_name = sia("Game");
    script_class.static_class_global_variable_name = sia("CodecClass");
    script_class.placeable = true;
    script_class.metadata_specifiers = {sia("NotBlueprintable")};
    script_class.metadata_values = {sia("true")};
    script_class.compose_onto_class_name = sia("UComposeTarget");

    precompiled::precompiled_enum enumeration;
    enumeration.name = sia("ECodec");
    enumeration.name_space = sia("Fixture");
    enumeration.names = {sia("First"), sia("Second")};
    enumeration.values = {-1, 2};

    precompiled::precompiled_global global;
    global.name = sia("CodecGlobal");
    global.name_space = sia("Fixture");
    global.type = integer_type();
    global.is_default_init = false;
    global.is_pure_constant = false;
    global.has_init_function = false;
    // InitFunc is serialized even when has_init_function is false.
    global.init_function = full_function("");

    precompiled::function_import imported;
    imported.imported_from_module = sia("Provider");
    imported.signature.name = sia("Imported");
    imported.signature.name_space = sia("Fixture");
    imported.signature.parameter_types = {integer_type()};
    imported.signature.parameter_flags = {1};
    imported.signature.parameter_default_args = {sia("1")};
    imported.signature.return_type = integer_type();

    precompiled::precompiled_module module;
    module.module_name = sia("CodecModule");
    module.functions = {full_function("GlobalFunction")};
    module.classes = {script_class};
    module.enums = {enumeration};
    module.global_variables = {global};
    module.function_imports = {imported};
    module.code_hash = 0x0102030405060708LL;
    module.imported_modules = {sia("Provider")};
    module.statics_class_name = sia("UCodecStatics");
    module.declared_events = {sia("FCodecEvent")};
    module.declared_delegates = {sia("FCodecDelegate")};
    module.script_relative_filename = sia("Fixture/Codec.as");
    module.post_init_functions = {sia("AfterInit")};
    fixture.modules.emplace_back(ansi_key("CodecModule"), std::move(module));

    precompiled::type_reference type_ref;
    type_ref.name = sia("UCodecFixture");
    type_ref.module = sia("CodecModule");
    type_ref.name_space = sia("Fixture");
    type_ref.sub_types = {integer_type()};
    fixture.type_references.emplace_back(0x1000, std::move(type_ref));
    fixture.type_id_reference_to_pointer.emplace_back(77, 0x1000);

    precompiled::function_reference function_ref;
    function_ref.name = sia("Method");
    function_ref.module = sia("CodecModule");
    function_ref.name_space = sia("Fixture");
    function_ref.is_const = true;
    function_ref.is_imported_decl = true;
    function_ref.is_method = true;
    function_ref.object_type = 0x1000;
    function_ref.parameter_types = {integer_type()};
    function_ref.return_type = integer_type();
    fixture.function_references.emplace_back(0x2000, std::move(function_ref));
    fixture.function_id_reference_to_pointer.emplace_back(88, 0x2000);

    precompiled::global_reference global_ref;
    global_ref.name = precompiled::archive_string{"Gr\xc3\xbc\xc3\x9f"};
    global_ref.module = sia("CodecModule");
    global_ref.name_space = sia("Fixture");
    global_ref.is_string = true;
    fixture.global_references.emplace_back(0x3000, std::move(global_ref));
    fixture.static_names = {sia("CodecName")};
    fixture.property_references.emplace_back(
        0x4001, precompiled::property_reference{sia("Health"), 77});
    return fixture;
}

bool roundtrip(const std::vector<std::uint8_t>& input, const char* const context) {
    precompiled::cache decoded;
    precompiled::codec_error error;
    if (!precompiled::decode(input.data(), input.size(), decoded, error)) {
        std::cerr << context << " decode failed at " << error.offset << " (" << error.field
                  << "): " << error.detail << '\n';
        return false;
    }
    std::vector<std::uint8_t> encoded;
    if (!precompiled::encode(decoded, encoded, error)) {
        std::cerr << context << " encode failed at " << error.offset << " (" << error.field
                  << "): " << error.detail << '\n';
        return false;
    }
    if (input != encoded) {
        std::cerr << context << " roundtrip was not byte exact\n";
        return false;
    }
    return true;
}

} // namespace

int main(const int argc, const char* const* const argv) {
    precompiled::codec_error error;
    const precompiled::cache fixture = full_fixture();
    std::vector<std::uint8_t> encoded;
    if (!precompiled::encode(fixture, encoded, error) || !roundtrip(encoded, "synthetic cache")) {
        return 1;
    }

    std::vector<std::uint8_t> trailing = encoded;
    trailing.push_back(0U);
    precompiled::cache rejected;
    if (precompiled::decode(trailing.data(), trailing.size(), rejected, error)) {
        std::cerr << "decoder accepted trailing cache bytes\n";
        return 2;
    }

    precompiled::cache invalid = fixture;
    invalid.static_names = {precompiled::archive_string{std::string("a\0b", 3U)}};
    std::vector<std::uint8_t> sentinel{0x7fU};
    if (precompiled::encode(invalid, sentinel, error) || sentinel != std::vector<std::uint8_t>{0x7fU}) {
        std::cerr << "encoder accepted an embedded NUL or changed output on failure\n";
        return 3;
    }

    precompiled::cache duplicate = fixture;
    duplicate.type_references.push_back(duplicate.type_references.front());
    if (precompiled::encode(duplicate, sentinel, error)) {
        std::cerr << "encoder accepted a duplicate TMap key\n";
        return 4;
    }

    precompiled::cache invalid_utf8 = fixture;
    invalid_utf8.global_references.front().second.name.bytes = std::string(1U, '\xff');
    if (precompiled::encode(invalid_utf8, sentinel, error)) {
        std::cerr << "encoder accepted invalid UTF-8 for a script string literal\n";
        return 5;
    }

    if (argc == 2) {
        std::ifstream stream(argv[1], std::ios::binary);
        if (!stream) {
            std::cerr << "could not open cache fixture\n";
            return 6;
        }
        const std::vector<std::uint8_t> real_cache{
            std::istreambuf_iterator<char>(stream), std::istreambuf_iterator<char>()};
        if (!roundtrip(real_cache, "external cache")) {
            return 7;
        }
    }

    std::cout << "precompiled cache codec smoke passed (" << encoded.size() << " bytes)\n";
    return 0;
}
