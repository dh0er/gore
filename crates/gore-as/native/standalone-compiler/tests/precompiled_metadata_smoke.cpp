#include "gore_as_standalone/precompiled_metadata.hpp"

#include <iostream>
#include <string>

namespace standalone = gore::as::standalone;
namespace precompiled = gore::as::standalone::precompiled;

namespace {

precompiled::precompiled_module output_fixture() {
    precompiled::precompiled_module module;
    module.module_name.bytes = "Game.Hero";
    precompiled::precompiled_class type;
    type.class_name.bytes = "AHero";
    precompiled::precompiled_property property;
    property.name.bytes = "Health";
    type.properties.push_back(std::move(property));
    precompiled::precompiled_property implicit_object;
    implicit_object.name.bytes = "ImplicitObject";
    implicit_object.type.type_info = 0x1234;
    implicit_object.type.is_object_handle = true;
    type.properties.push_back(std::move(implicit_object));
    precompiled::precompiled_property implicit_scalar;
    implicit_scalar.name.bytes = "ImplicitScalar";
    implicit_scalar.type.token_type = 4;
    type.properties.push_back(std::move(implicit_scalar));
    precompiled::precompiled_function method;
    method.function_name.bytes = "Run_Implementation";
    method.is_const_method = true;
    method.is_no_op = true;
    type.methods.push_back(std::move(method));
    module.classes.push_back(std::move(type));
    precompiled::precompiled_function global;
    global.function_name.bytes = "GlobalRun";
    global.is_no_op = true;
    module.functions.push_back(std::move(global));
    return module;
}

precompiled::class_generator_capability_table capability_fixture() {
    precompiled::class_generator_capability_table table;
    precompiled::class_generator_class_capabilities type;
    type.class_name = "AHero";
    type.properties = {
        {"Health", "int32", true, false, false},
        {"ImplicitObject", "UObject", true, false, true},
        {"ImplicitScalar", "int32", true, false, false},
    };
    table.classes.push_back(std::move(type));
    return table;
}

standalone::lexical_module_description descriptor_fixture() {
    standalone::lexical_module_description module;
    module.module_name = "Game.Hero";
    module.code_hash = 42;
    module.imported_modules = {"Game.Base"};
    module.post_init_functions = {"InitializeHero"};
    module.statics_class_name = "UHeroStatics";
    module.code.push_back({
        "Game/Hero.as", "C:/sealed/Script/Game/Hero.as", {}});
    module.delegates.push_back({"FHeroEvent", {}, 1U, true});

    standalone::preprocessed_class_description type;
    type.class_name = "AHero";
    type.super_class = "AActor";
    type.code_super_class = "/Script/Engine.Actor";
    type.super_is_code_class = true;
    type.abstract = true;
    type.config_name = "Game";
    type.compose_onto_class = "/Script/Game.HeroBase";
    type.static_class_global_variable_name = "AHero::StaticClass";
    type.metadata.push_back({"Category", "Hero", -1});
    standalone::preprocessed_property_description property;
    property.property_name = "Health";
    property.blueprint_readable = true;
    property.replicated = true;
    property.replication_condition = 3;
    property.metadata.push_back({"ClampMin", "0", -1});
    type.properties.push_back(std::move(property));
    standalone::preprocessed_function_description method;
    method.function_name = "Run";
    method.script_function_name = "Run_Implementation";
    method.blueprint_callable = true;
    method.blueprint_event = true;
    method.thread_safe = true;
    method.metadata.push_back({"Category", "Hero", -1});
    type.methods.push_back(std::move(method));
    module.classes.push_back(std::move(type));

    standalone::preprocessed_class_description statics;
    statics.class_name = "UHeroStatics";
    statics.is_statics_class = true;
    standalone::preprocessed_function_description global;
    global.function_name = "GlobalRun";
    global.script_function_name = "GlobalRun";
    global.blueprint_callable = true;
    global.is_static = true;
    statics.methods.push_back(std::move(global));
    module.classes.push_back(std::move(statics));
    return module;
}

} // namespace

int main() {
    precompiled::precompiled_module output = output_fixture();
    const standalone::lexical_module_description description = descriptor_fixture();
    const precompiled::class_generator_capability_table capabilities =
        capability_fixture();
    const auto projected =
        precompiled::project_preprocessed_metadata(
            description, output, false, &capabilities);
    const auto& type = output.classes[0];
    const auto& property = type.properties[0];
    const auto& implicit_object = type.properties[1];
    const auto& implicit_scalar = type.properties[2];
    const auto& method = type.methods[0];
    const auto& global = output.functions[0];
    if (!projected.ok || output.imported_modules.size() != 1U ||
        output.post_init_functions.size() != 1U ||
        output.statics_class_name.bytes != "UHeroStatics" ||
        output.code_hash != 42 ||
        output.script_relative_filename.bytes != "Game/Hero.as" ||
        output.declared_events.size() != 1U ||
        !type.is_in_preprocessor || type.super_class.bytes != "AActor" ||
        type.code_super_class.bytes != "/Script/Engine.Actor" ||
        type.compose_onto_class_name.bytes != "/Script/Game.HeroBase" ||
        type.metadata_specifiers.size() != 1U ||
        !property.is_unreal_property || !property.blueprint_readable ||
        !property.replicated || property.replication_condition != 3 ||
        !implicit_object.is_unreal_property || !implicit_object.transient ||
        implicit_scalar.is_unreal_property ||
        !method.is_unreal_function ||
        method.unreal_function_name.bytes != "Run" ||
        !method.blueprint_event || !method.thread_safe ||
        !method.is_const_method || !method.is_no_op ||
        !global.is_unreal_function || !global.is_static || global.is_no_op) {
        std::cerr << "preprocessed metadata projection was incomplete: "
                  << projected.detail << '\n';
        return 1;
    }

    precompiled::precompiled_module struct_output = output_fixture();
    standalone::lexical_module_description struct_description = description;
    struct_description.classes[0].is_struct = true;
    const auto projected_struct = precompiled::project_preprocessed_metadata(
        struct_description, struct_output, false, &capabilities);
    if (!projected_struct.ok ||
        !struct_output.classes[0].properties[1].is_unreal_property ||
        struct_output.classes[0].properties[1].transient ||
        !struct_output.classes[0].properties[2].is_unreal_property ||
        struct_output.classes[0].properties[2].transient) {
        std::cerr << "captured-false struct transient policy was not applied\n";
        return 2;
    }
    precompiled::precompiled_module transient_struct_output = output_fixture();
    const auto projected_transient_struct =
        precompiled::project_preprocessed_metadata(
            struct_description, transient_struct_output, true, &capabilities);
    if (!projected_transient_struct.ok ||
        !transient_struct_output.classes[0].properties[1].is_unreal_property ||
        !transient_struct_output.classes[0].properties[1].transient ||
        !transient_struct_output.classes[0].properties[2].is_unreal_property ||
        !transient_struct_output.classes[0].properties[2].transient) {
        std::cerr << "captured-true struct transient policy was not applied\n";
        return 3;
    }

    precompiled::precompiled_module untouched = output_fixture();
    standalone::lexical_module_description broken = description;
    broken.classes[0].properties[0].property_name = "Missing";
    const auto rejected =
        precompiled::project_preprocessed_metadata(
            broken, untouched, false, &capabilities);
    if (rejected.ok || untouched.classes[0].is_in_preprocessor ||
        untouched.classes[0].properties[0].is_unreal_property ||
        untouched.imported_modules.size() != 0U) {
        std::cerr << "failed metadata projection was not atomic\n";
        return 4;
    }

    precompiled::precompiled_module unsupported = output_fixture();
    precompiled::class_generator_capability_table unsupported_capabilities =
        capability_fixture();
    unsupported_capabilities.classes[0].properties[2].can_create_property = false;
    unsupported_capabilities.classes[0].properties[2].type_declaration =
        "FNumberFormattingOptions";
    const auto unsupported_result = precompiled::project_preprocessed_metadata(
        struct_description, unsupported, false, &unsupported_capabilities);
    if (unsupported_result.ok || unsupported.classes[0].is_in_preprocessor ||
        unsupported.classes[0].properties[0].is_unreal_property ||
        unsupported.classes[0].properties[2].is_unreal_property ||
        !unsupported_result.is_compile_diagnostic ||
        unsupported_result.diagnostic_source != "C:/sealed/Script/Game/Hero.as" ||
        unsupported_result.diagnostic_line != struct_description.classes[0].line ||
        unsupported_result.diagnostic_column != 1U ||
        unsupported_result.detail !=
            "Property ImplicitScalar with type FNumberFormattingOptions is in a context where a UPROPERTY must be generated for GC reasons, but the property type is not supported by UPROPERTY.") {
        std::cerr << "unsupported required property was not rejected atomically\n";
        return 5;
    }

    precompiled::precompiled_module never_gc = output_fixture();
    precompiled::class_generator_capability_table never_gc_capabilities =
        capability_fixture();
    never_gc_capabilities.classes[0].properties[2].never_requires_gc = true;
    const auto never_gc_result = precompiled::project_preprocessed_metadata(
        struct_description, never_gc, false, &never_gc_capabilities);
    if (!never_gc_result.ok ||
        never_gc.classes[0].properties[2].is_unreal_property) {
        std::cerr << "NeverRequiresGC struct property was synthesized\n";
        return 6;
    }

    precompiled::precompiled_module requires_property = output_fixture();
    precompiled::class_generator_capability_table requires_property_capabilities =
        capability_fixture();
    requires_property_capabilities.classes[0].properties[2].never_requires_gc = true;
    requires_property_capabilities.classes[0].properties[2].requires_property = true;
    const auto requires_property_result = precompiled::project_preprocessed_metadata(
        struct_description, requires_property, false, &requires_property_capabilities);
    if (!requires_property_result.ok ||
        !requires_property.classes[0].properties[2].is_unreal_property) {
        std::cerr << "RequiresProperty did not override NeverRequiresGC for a struct\n";
        return 7;
    }

    std::cout << "precompiled metadata projection smoke passed\n";
    return 0;
}
