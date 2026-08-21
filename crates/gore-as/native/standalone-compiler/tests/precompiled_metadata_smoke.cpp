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
    precompiled::precompiled_function method;
    method.function_name.bytes = "Run_Implementation";
    method.is_const_method = true;
    method.is_no_op = true;
    type.methods.push_back(std::move(method));
    module.classes.push_back(std::move(type));
    precompiled::precompiled_function global;
    global.function_name.bytes = "GlobalRun";
    module.functions.push_back(std::move(global));
    return module;
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
    const auto projected =
        precompiled::project_preprocessed_metadata(description, output);
    const auto& type = output.classes[0];
    const auto& property = type.properties[0];
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
        type.metadata_specifiers.size() != 1U ||
        !property.is_unreal_property || !property.blueprint_readable ||
        !property.replicated || property.replication_condition != 3 ||
        !method.is_unreal_function ||
        method.unreal_function_name.bytes != "Run" ||
        !method.blueprint_event || !method.thread_safe ||
        !method.is_const_method || !method.is_no_op ||
        !global.is_unreal_function || !global.is_static) {
        std::cerr << "preprocessed metadata projection was incomplete: "
                  << projected.detail << '\n';
        return 1;
    }

    precompiled::precompiled_module untouched = output_fixture();
    standalone::lexical_module_description broken = description;
    broken.classes[0].properties[0].property_name = "Missing";
    const auto rejected =
        precompiled::project_preprocessed_metadata(broken, untouched);
    if (rejected.ok || untouched.classes[0].is_in_preprocessor ||
        untouched.classes[0].properties[0].is_unreal_property ||
        untouched.imported_modules.size() != 0U) {
        std::cerr << "failed metadata projection was not atomic\n";
        return 2;
    }

    std::cout << "precompiled metadata projection smoke passed\n";
    return 0;
}
