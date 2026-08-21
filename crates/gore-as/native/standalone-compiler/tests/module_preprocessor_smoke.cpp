#include "gore_as_standalone/module_preprocessor.hpp"

#include <iostream>
#include <string>
#include <vector>

namespace standalone = gore::as::standalone;

namespace {

int fail(const char* message) {
    std::cerr << message << '\n';
    return 1;
}

standalone::preprocessor_source source(
    std::string relative_path,
    std::string code) {
    return {
        relative_path,
        "C:/sealed/Script/" + relative_path,
        std::move(code)};
}

bool is_blank_except_whitespace(const std::string& value) {
    for (const char character : value) {
        if (character != ' ' && character != '\t' &&
            character != '\r' && character != '\n') return false;
    }
    return true;
}

const standalone::preprocessor_metadata* metadata(
    const std::vector<standalone::preprocessor_metadata>& entries,
    const std::string& name,
    const std::int32_t subject_index = -1) {
    for (const auto& entry : entries) {
        if (entry.name == name && entry.subject_index == subject_index) return &entry;
    }
    return nullptr;
}

} // namespace

int main() {
    std::int64_t donor_hash = 0;
    if (!standalone::compute_processed_code_hash_utf8("abc", donor_hash) ||
        static_cast<std::uint64_t>(donor_hash) != 0xaff0f2a2f8b32731ULL ||
        standalone::compute_processed_code_hash_utf8(
            std::string("\xc0\x80", 2U), donor_hash)) {
        return fail("UTF-16LE XXH64 code hashing drifted from the pinned donor");
    }
    const auto empty = standalone::preprocess_lexical_module_graph({}, {});
    if (!empty.ok || !empty.modules.empty() || !empty.diagnostics.empty()) {
        return fail("empty donor source set did not remain a successful no-op");
    }

    standalone::preprocessor_options explicit_imports;
    explicit_imports.automatic_imports = false;
    explicit_imports.flags = {
        {"DEFINED_FALSE", false},
        {"ENABLED", true},
        {"DISABLED", false},
    };

    std::vector<standalone::preprocessor_source> sources;
    sources.push_back(source("Game/Consumer.as", R"AS(import Game.Provider;
import void ImportedCall() from "Native";
namespace Outer
{
    import Game.NamespaceProvider;
}
#ifdef DEFINED_FALSE
int PresentBecauseDefined() { return 1; }
#endif
#if DISABLED
int RemovedA() { return 0; }
#elif ENABLED
int Selected() { return 42; }
#else
int RemovedB() { return 0; }
#endif
#ifndef MISSING
int MissingIsAbsent() { return 1; }
#endif
const string Literal = "import Not.A.Module;";
// import Not.A.Comment;
)AS"));
    sources.push_back(source(
        "Game/Provider.as", "int Provider() { return 20; }\n"));
    sources.push_back(source(
        "Game/NamespaceProvider.as", "int NamespaceProvider() { return 22; }\n"));

    const auto explicit_result =
        standalone::preprocess_lexical_module_graph(explicit_imports, sources);
    if (!explicit_result.ok || !explicit_result.diagnostics.empty()) {
        return fail("explicit-import lexical preprocessing failed");
    }
    if (explicit_result.modules.size() != 3U ||
        explicit_result.modules[0].module_name != "Game.Provider" ||
        explicit_result.modules[1].module_name != "Game.NamespaceProvider" ||
        explicit_result.modules[2].module_name != "Game.Consumer") {
        return fail("explicit imports did not produce donor dependency order");
    }
    const auto& consumer = explicit_result.modules[2];
    if (consumer.imported_modules !=
        std::vector<std::string>{"Game.Provider", "Game.NamespaceProvider"}) {
        return fail("top-level module imports were not discovered exactly");
    }
    const std::string& conditioned = consumer.code[0].conditioned_code;
    if (conditioned.find("PresentBecauseDefined") == std::string::npos ||
        conditioned.find("Selected") == std::string::npos ||
        conditioned.find("MissingIsAbsent") == std::string::npos ||
        conditioned.find("RemovedA") != std::string::npos ||
        conditioned.find("RemovedB") != std::string::npos ||
        conditioned.find("import void ImportedCall()") == std::string::npos ||
        conditioned.find("import Not.A.Module;") == std::string::npos) {
        return fail("conditionals, function imports, strings or comments drifted");
    }
    const std::size_t first_line_end = conditioned.find('\n');
    if (first_line_end == std::string::npos ||
        !is_blank_except_whitespace(conditioned.substr(0U, first_line_end))) {
        return fail("manual module import was not blanked with layout preservation");
    }

    standalone::preprocessor_options automatic = explicit_imports;
    automatic.automatic_imports = true;
    const auto automatic_result =
        standalone::preprocess_lexical_module_graph(automatic, sources);
    if (!automatic_result.ok || automatic_result.modules.size() != 3U ||
        automatic_result.modules[0].module_name != "Game.Consumer" ||
        !automatic_result.modules[0].imported_modules.empty() ||
        automatic_result.modules[0].code[0].conditioned_code.find(
            "import Game.Provider;") != 0U) {
        return fail("automatic-import mode did not preserve donor input behavior");
    }

    const auto unknown = standalone::preprocess_lexical_module_graph(
        automatic,
        {source("Bad/Unknown.as", "#if UNKNOWN\nint X;\n#endif\n")});
    if (unknown.ok || unknown.diagnostics.size() != 1U ||
        unknown.diagnostics[0].row != 1U ||
        unknown.diagnostics[0].message != "Invalid preprocessor condition: UNKNOWN") {
        return fail("unknown preprocessor flag did not fail with exact diagnostics");
    }

    const auto cycle = standalone::preprocess_lexical_module_graph(
        explicit_imports,
        {source("Cycle/A.as", "import Cycle.B;\n"),
         source("Cycle/B.as", "import Cycle.A;\n")});
    if (cycle.ok || cycle.modules.size() != 2U || cycle.diagnostics.size() != 3U ||
        cycle.diagnostics[0].message !=
            "Detected circular import of module Cycle.A. Import chain:" ||
        cycle.diagnostics[1].message != "   => Cycle.B" ||
        cycle.diagnostics[2].message != "   => Cycle.A") {
        return fail("circular import diagnostics or recovery order drifted");
    }

    const auto unclosed = standalone::preprocess_lexical_module_graph(
        automatic,
        {source("Bad/Unclosed.as", "#if ENABLED\nint X;\n")});
    if (unclosed.ok || unclosed.diagnostics.size() != 1U ||
        unclosed.diagnostics[0].row != 3U ||
        unclosed.diagnostics[0].message !=
            "Preceding preprocessor #if/#ifdef/#else was not closed, missing #endif.") {
        return fail("unclosed conditional diagnostic drifted");
    }

    standalone::preprocessor_options dialect;
    dialect.automatic_imports = true;
    dialect.static_classes = standalone::static_class_mode::deprecated;
    dialect.default_function_blueprint_callable = false;
    dialect.default_property_edit =
        standalone::property_edit_specifier::edit_instance_only;
    dialect.default_struct_property_edit =
        standalone::property_edit_specifier::not_editable;
    dialect.default_property_blueprint =
        standalone::property_blueprint_specifier::blueprint_read_only;
    dialect.script_float_is_float64 = true;
    dialect.static_names = {"Existing"};
    dialect.blueprint_event_argument_specializations = {"int32"};
    dialect.native_super_types = {
        {"AActor", "/Script/Engine.Actor", 0U, standalone::native_super_kind::actor, false},
        {"UObject", "/Script/CoreUObject.Object", 0U, standalone::native_super_kind::other_uobject, false},
    };
    const auto dialect_result = standalone::preprocess_lexical_module_graph(
        dialect,
        {source("Game/Dialect.as", R"AS(namespace Demo
{
UCLASS(Abstract, NotPlaceable, Config=Game, Meta=(DisplayName="Hero"))
class AHero : AActor
{
    UPROPERTY(BlueprintReadOnly, VisibleDefaultsOnly, ReplicatedUsing=OnRep_Health, SaveGame, Meta=(ClampMin="0"))
    int32 Health = 10;

    UFUNCTION(BlueprintCallable, BlueprintEvent, Category="Demo")
    int32 Compute(int32 Value, const FString& Label) const { return Value; }

    default Health = 7; // stripped from DefaultsCode only
}

USTRUCT(Blueprintable)
struct FState
{
    UPROPERTY(NotReplicated, BlueprintHidden)
    int32 Count;
    default Count = 3;
}

class AChild : AHero
{
}

UENUM(DisplayName="Mode", Meta=(Bitflags=""))
enum EMode
{
    One UMETA(DisplayName="First"),
    Two,
}

delegate int32 FCompute(int32 Value);
event void FChanged(int32 Value);
}

FName ExistingName = n"Existing";
FName AddedName = n"Added";
FString Message = f"Value {AddedName=} {42:.2f} {{ok}}";
for (const FThing& Item : Things) { Use(Item); }
asset Settings of UMySettings
)AS")});
    if (!dialect_result.ok || !dialect_result.diagnostics.empty() ||
        dialect_result.modules.size() != 1U ||
        dialect_result.static_names !=
            std::vector<std::string>{"Existing", "Added", "Compute"}) {
        return fail("dialect frontend or manager-global static-name order drifted");
    }
    const auto& dialect_module = dialect_result.modules[0];
    if (dialect_module.classes.size() != 3U ||
        dialect_module.enums.size() != 1U ||
        dialect_module.delegates.size() != 2U ||
        dialect_module.post_init_functions != std::vector<std::string>{"GetSettings"}) {
        return fail("type, delegate or literal-asset discovery drifted");
    }
    const auto& hero = dialect_module.classes[0];
    if (hero.class_name != "AHero" || hero.name_space != "Demo" ||
        hero.super_class != "AActor" || !hero.abstract || hero.placeable ||
        hero.config_name != "Game" ||
        hero.defaults_code.find("Health = 7;") != 0U ||
        !is_blank_except_whitespace(hero.defaults_code.substr(11U)) ||
        hero.properties.size() != 1U || hero.methods.size() != 1U ||
        metadata(hero.metadata, "DisplayName") == nullptr ||
        metadata(hero.metadata, "DisplayName")->value != "Hero") {
        return fail("class declaration, defaults or class specifiers drifted");
    }
    const auto& health = hero.properties[0];
    if (health.property_name != "Health" || health.literal_type != "int32" ||
        !health.blueprint_readable || health.blueprint_writable ||
        !health.editable_on_defaults || health.editable_on_instance ||
        !health.replicated || !health.rep_notify || !health.save_game ||
        metadata(health.metadata, "ReplicatedUsing") == nullptr ||
        metadata(health.metadata, "ClampMin") == nullptr) {
        return fail("UPROPERTY defaults, specifiers or metadata drifted");
    }
    const auto& compute = hero.methods[0];
    if (compute.function_name != "Compute" ||
        compute.script_function_name != "Compute_Implementation" ||
        !compute.blueprint_callable || !compute.blueprint_event) {
        return fail("UFUNCTION descriptor or event wrapper naming drifted");
    }
    const auto& state = dialect_module.classes[1];
    if (!state.is_struct || state.defaults_code != "Count = 3;" ||
        state.properties.size() != 1U ||
        !state.properties[0].skip_replication ||
        state.properties[0].blueprint_readable ||
        state.properties[0].blueprint_writable) {
        return fail("struct defaults or NotReplicated handling drifted");
    }
    const auto& child = dialect_module.classes[2];
    if (child.class_name != "AChild" || child.super_class != "AHero" ||
        child.super_is_code_class || child.code_super_class != "/Script/Engine.Actor" ||
        child.code_super_kind != standalone::native_super_kind::actor) {
        return fail("script-to-script native root resolution drifted");
    }
    const auto& enumeration = dialect_module.enums[0];
    if (enumeration.enum_name != "EMode" || enumeration.name_space != "Demo" ||
        metadata(enumeration.metadata, "DisplayName") == nullptr ||
        metadata(enumeration.metadata, "Bitflags") == nullptr ||
        metadata(enumeration.metadata, "DisplayName", 0) == nullptr ||
        metadata(enumeration.metadata, "DisplayName", 0)->value != "First") {
        return fail("UENUM or UMETA materialization drifted");
    }
    const std::string& dialect_code = dialect_module.code[0].conditioned_code;
    if (dialect_code.find("UCLASS(") != std::string::npos ||
        dialect_code.find("UPROPERTY(") != std::string::npos ||
        dialect_code.find("UFUNCTION(") != std::string::npos ||
        dialect_code.find("UENUM(") != std::string::npos ||
        dialect_code.find("UMETA(") != std::string::npos ||
        dialect_code.find("Compute_Implementation") == std::string::npos ||
        dialect_code.find("__Evt_PushArgument__int32(Value)") == std::string::npos ||
        dialect_code.find("__Evt_Execute(this, __STATIC_NAME(2))") == std::string::npos ||
        dialect_code.find("__STATIC_NAME(0)") == std::string::npos ||
        dialect_code.find("__STATIC_NAME(1)") == std::string::npos ||
        dialect_code.find("struct FCompute {") == std::string::npos ||
        dialect_code.find("struct FChanged {") == std::string::npos ||
        dialect_code.find("__auto_constref_type") == std::string::npos ||
        dialect_code.find("FString::ApplyFormat") == std::string::npos ||
        dialect_code.find("__Asset_Settings") == std::string::npos ||
        dialect_code.find("StaticClass() __generated deprecated") == std::string::npos ||
        dialect_code.find("class AHero : AActor") != std::string::npos ||
        dialect_code.find("class AChild : AHero") == std::string::npos ||
        dialect_code.find("Spawn(const FVector& Location") == std::string::npos) {
        return fail("dialect source lowering or generated code drifted");
    }

    standalone::preprocessor_options forbidden_options;
    forbidden_options.native_super_types = {
        {"AForbidden", "/Script/Test.Forbidden", 0U, standalone::native_super_kind::actor, true},
        {"UObject", "/Script/CoreUObject.Object", 0U, standalone::native_super_kind::other_uobject, false},
    };
    const auto forbidden = standalone::preprocess_lexical_module_graph(
        forbidden_options,
        {source("Bad/Forbidden.as", "class ABad : AForbidden {}\n")});
    if (forbidden.ok || forbidden.diagnostics.size() != 1U ||
        forbidden.diagnostics[0].message !=
            "Class ABad cannot inherit from C++ class AForbidden which specifies "
            "CannotDeriveAngelscript meta") {
        return fail("native CannotDeriveAngelscript gate drifted");
    }

    standalone::preprocessor_options condition_options;
    condition_options.flags = {{"DISABLED", false}, {"FEATURE", true}};
    condition_options.native_super_types = {
        {"UObject", "/Script/CoreUObject.Object", 0U, standalone::native_super_kind::other_uobject, false},
    };
    const auto condition_result = standalone::preprocess_lexical_module_graph(
        condition_options,
        {source("Bad/ConditionalMacro.as", R"AS(class AConditional
{
#if FEATURE
    UPROPERTY()
    int32 Allowed;
#endif
#if !DISABLED
    UPROPERTY()
    int32 Rejected;
#endif
}
)AS")});
    if (condition_result.ok || condition_result.diagnostics.size() != 1U ||
        condition_result.diagnostics[0].message !=
            "Cannot put a UPROPERTY or UFUNCTION inside preprocessor conditions other "
            "than EDITOR or flags declared in configuration.") {
        return fail("reflection-macro conditional gate drifted");
    }

    standalone::preprocessor_options base_options;
    base_options.native_super_types = {
        {"AActor", "/Script/Engine.Actor", 0U, standalone::native_super_kind::actor, false},
        {"UObject", "/Script/CoreUObject.Object", 0U, standalone::native_super_kind::other_uobject, false},
    };
    const std::vector<standalone::preprocessor_base_module> base_modules = {{
        "Game.Base",
        {{"ABase", "", "AActor", "/Script/Engine.Actor", false, false}},
    }};
    const auto base_child = standalone::preprocess_lexical_module_graph(
        base_options,
        {source("Game/Child.as", "class AChild : ABase {}\n")},
        base_modules);
    if (!base_child.ok || base_child.modules.size() != 1U ||
        base_child.modules[0].classes.size() != 1U ||
        base_child.modules[0].classes[0].super_is_code_class ||
        base_child.modules[0].classes[0].code_super_class != "/Script/Engine.Actor" ||
        base_child.modules[0].classes[0].code_super_kind !=
            standalone::native_super_kind::actor ||
        base_child.modules[0].code[0].conditioned_code.find(
            "Spawn(const FVector& Location") == std::string::npos) {
        return fail("decoded base-class ancestry was not preserved for an add overlay");
    }

    auto edit_source = source("Game/Base.as", "class ABase : UObject {}\n");
    edit_source.overlay_operation = standalone::preprocessor_source::operation::edit;
    edit_source.module_name = "Game.Base";
    const auto base_edit = standalone::preprocess_lexical_module_graph(
        base_options, {edit_source}, base_modules);
    if (!base_edit.ok || base_edit.modules.size() != 1U ||
        base_edit.modules[0].classes.size() != 1U ||
        !base_edit.modules[0].classes[0].super_is_code_class ||
        base_edit.modules[0].classes[0].code_super_class !=
            "/Script/CoreUObject.Object") {
        return fail("edit overlay did not replace its authoritative base module");
    }
    const auto base_collision = standalone::preprocess_lexical_module_graph(
        base_options,
        {source("Game/Base.as", "class AReplacement : UObject {}\n")},
        base_modules);
    if (base_collision.ok || base_collision.diagnostics.size() != 1U ||
        base_collision.diagnostics[0].message !=
            "add overlay collides with a base module") {
        return fail("add/edit base-module collision contract drifted");
    }

    standalone::preprocessor_options haze_options = base_options;
    haze_options.angelscript_haze = true;
    const auto haze = standalone::preprocess_lexical_module_graph(
        haze_options,
        {source("Game/Haze.as", R"AS(class AHaze : UObject
{
    UFUNCTION(NetFunction)
    void NetCall() {}
    UFUNCTION(DevFunction)
    void DevCall() {}
}
)AS")});
    if (!haze.ok || haze.modules[0].classes[0].methods.size() != 2U ||
        !haze.modules[0].classes[0].methods[0].net_function ||
        !haze.modules[0].classes[0].methods[1].dev_function) {
        return fail("Haze-only function specifiers drifted");
    }
    const auto non_haze_dev = standalone::preprocess_lexical_module_graph(
        base_options,
        {source("Bad/Dev.as", R"AS(class ADev : UObject
{
    UFUNCTION(DevFunction)
    void DevCall() {}
}
)AS")});
    if (non_haze_dev.ok || non_haze_dev.diagnostics.size() != 1U ||
        non_haze_dev.diagnostics[0].message.find("Unknown function specifier DevFunction") != 0U) {
        return fail("non-Haze dialect accepted a Haze-only function specifier");
    }
    standalone::preprocessor_options validation_options = base_options;
    validation_options.enforce_server_rpc_validation = true;
    const auto missing_validation = standalone::preprocess_lexical_module_graph(
        validation_options,
        {source("Bad/Server.as", R"AS(class AServer : UObject
{
    UFUNCTION(Server)
    void ServerCall() {}
}
)AS")});
    if (missing_validation.ok || missing_validation.diagnostics.size() != 1U ||
        missing_validation.diagnostics[0].message !=
            "UFUNCTION() ServerCall is marked as Server but does not have the "
            "WithValidation property specified!") {
        return fail("server RPC validation profile switch drifted");
    }

    std::cout << "G1R preprocessor smoke covered conditionals, graph order, reflection "
                 "macros, dialect switches, base ancestry, delegates and source lowering\n";
    return 0;
}
