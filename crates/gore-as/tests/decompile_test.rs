use gore_as::cache::cfg;
use gore_as::cache::decompile::decompile_function;
use gore_as::cache::disasm::{disassemble, listing};
use gore_as::cache::refs::RefResolver;
use gore_as::cache::walk_modules::collect_function_bytecodes;

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../work/reversing/gore-as/samples"
);

fn read_sample(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{SAMPLES}/{name}")).ok()
}

#[test]
fn decompiles_richtest_method1() {
    let Some(b) = read_sample("PrecompiledScript.richtest.Cache") else {
        eprintln!("skip: richtest sample not present");
        return;
    };
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let m = funcs
        .iter()
        .find(|f| f.func.ends_with("::method1"))
        .expect("method1");
    let src = decompile_function(m, &refs);
    eprintln!("{src}");
    // source was: int method1(int a, float b) { return a + field1; }
    assert!(src.contains("return"), "has return");
    assert!(src.contains("field1"), "resolves member field1");
    assert!(
        src.contains("a + this.field1") || src.contains("(a + this.field1)"),
        "reconstructs a + this.field1:\n{src}"
    );
}

#[test]
fn dump_branchtest() {
    let Some(b) = read_sample("PrecompiledScript.branchtest.Cache") else {
        eprintln!("skip: branchtest sample not present");
        return;
    };
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let f = funcs
        .iter()
        .find(|f| f.func.contains("GoreBranchTest"))
        .expect("GoreBranchTest");
    let instrs = disassemble(&f.bytecode).unwrap();
    eprintln!("=== DISASM {} ===\n{}", f.func, listing(&instrs));
    let g = cfg::build(&instrs);
    eprintln!(
        "=== CFG: {} blocks (back_edge={}) ===",
        g.blocks.len(),
        g.has_back_edge()
    );
    for bb in &g.blocks {
        eprintln!(
            "  block @{} instrs[{}..{}] -> {:?}",
            bb.start_dw, bb.instr_lo, bb.instr_hi, bb.succs
        );
    }
    eprintln!(
        "=== DECOMPILE (linear) ===\n{}",
        decompile_function(f, &refs)
    );
    eprintln!(
        "=== DECOMPILE (structured) ===\n{}",
        gore_as::cache::structure::decompile(f, &refs)
    );
}

#[test]
fn structures_branchtest() {
    let Some(b) = read_sample("PrecompiledScript.branchtest.Cache") else {
        return;
    };
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let f = funcs
        .iter()
        .find(|f| f.func.contains("GoreBranchTest"))
        .unwrap();
    let src = gore_as::cache::structure::decompile(f, &refs);
    // source: for(i<n) sum+=i; if(sum>100) sum=100 else sum+1; while(sum>0) sum-=5; return sum
    assert!(src.contains("while (local_3 < n)"), "for-loop:\n{src}");
    assert!(src.contains("if (local_1 > 100)"), "if cond:\n{src}");
    assert!(src.contains("else"), "else branch:\n{src}");
    assert!(src.contains("while (local_1 > 0)"), "while-loop:\n{src}");
    assert!(src.contains("return local_1;"), "return:\n{src}");
}

/// batch-49 recovery locks: decompile named functions from the real vanilla cache_A sample
/// (gitignored; graceful-skip when absent) and assert the recovered statement is present.
/// These guard the batch-49a bool-member-store and batch-49c opIndex-getter-into-null-compare
/// arms against future regression. Uses `structure::decompile` (ret_ty-aware since batch-49b).
#[test]
fn batch49_recoveries_present() {
    let Some(b) = read_sample("cache_A.Cache") else {
        eprintln!("skip: cache_A.Cache sample not present");
        return;
    };
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let decomp = |needle: &str| -> String {
        funcs
            .iter()
            .find(|f| f.func.contains(needle))
            .map(|f| gore_as::cache::structure::decompile(f, &refs))
            .unwrap_or_default()
    };
    // batch-49a: `this.bShouldExitState = true` (WRTV1 into a UE-bool member) was dropped.
    let og = decomp("UAIState_UseFreepoint::OnGracefulExitRequested");
    assert!(
        og.contains("bShouldExitState = (local_1 != 0)"),
        "batch-49a bool member-store dropped:\n{og}"
    );
    // batch-49c: `local_4 = m_SpawnedAreas[type]` (opIndex getter into a null-compared slot)
    // was dropped, leaving the null-guard reading an uninitialised slot.
    let contains = decomp("UDemonAreaRegisterComponent::Contains");
    assert!(
        contains.contains("local_4 = this.m_SpawnedAreas.opIndex("),
        "batch-49c opIndex-getter-into-null-compare dropped:\n{contains}"
    );
    assert!(
        contains.contains("if (local_4 == nullptr)"),
        "batch-49c null-guard should read the recovered element:\n{contains}"
    );
}

/// Structured decompile must not panic or hang across many real functions (loop guard).
#[test]
fn structured_real_cache_no_panic() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let mut chars = 0usize;
    for f in funcs.iter().take(20000) {
        chars += gore_as::cache::structure::decompile(f, &refs).len();
    }
    eprintln!("structured 20000 funcs, {chars} chars total");
}

/// Tally opcode frequency across all real function bytecode (to prioritize op coverage).
#[test]
fn opcode_frequency() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let mut freq: std::collections::HashMap<&'static str, u64> = std::collections::HashMap::new();
    for f in &funcs {
        if let Ok(ins) = gore_as::cache::disasm::disassemble(&f.bytecode) {
            for i in &ins {
                *freq.entry(i.op.name).or_insert(0) += 1;
            }
        }
    }
    let mut v: Vec<_> = freq.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("=== top 45 opcodes by frequency ===");
    for (name, n) in v.iter().take(45) {
        eprintln!("{n:>9}  {name}");
    }
}

/// Measure body-recovery rate across ALL real functions: a body is "clean" if it has no
/// raw `// OPCODE` annotation / disasm error (i.e. fully decompiled, no stub needed).
#[test]
fn body_recovery_rate() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).unwrap();
    let refs = RefResolver::build(&b).unwrap();
    let funcs = collect_function_bytecodes(&b).unwrap();
    let total = funcs.len();
    let mut clean = 0usize;
    for f in &funcs {
        let body = gore_as::cache::structure::body_statements(f, &refs, 1);
        let stub = body.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with("// ") && {
                let w = t
                    .trim_start_matches("// ")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                w.chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false)
                    && w.chars().all(|c| c.is_ascii_alphanumeric())
            }
        });
        if !stub {
            clean += 1;
        }
    }
    eprintln!(
        "body recovery: {clean}/{total} functions fully decompiled ({}%)",
        clean * 100 / total.max(1)
    );
}

/// Decompile a couple of real modules (env-gated) and print them; must not panic.
#[test]
fn decompiles_real_targets() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let refs = RefResolver::build(&b).expect("resolver");
    let funcs = collect_function_bytecodes(&b).expect("collect");
    for needle in ["LevelFormula", "GE_Kill"] {
        if let Some(f) = funcs.iter().find(|f| f.func.contains(needle)) {
            eprintln!("==== {} ====\n{}", f.func, decompile_function(f, &refs));
        }
    }
    // decompiling every function must not panic
    for f in funcs.iter().take(2000) {
        let _ = decompile_function(f, &refs);
    }
}

/// Real-cache lock for the Hazelight `Thiscall1` physical-frame rule. The optimized
/// `TArray::Last()` call consumes a compiler-pushed zero even though it renders with no
/// source arguments; that zero must not displace the deferred FName argument of the outer
/// delegate bind.
#[test]
fn thiscall1_preserves_deferred_static_name_in_real_cache() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let f = funcs
        .iter()
        .find(|f| {
            f.func
                .contains("UAbilityTask_Interaction_Human_Drink::SetupTransitions_Implementation")
        })
        .expect("hotfix corpus contains Drink::SetupTransitions_Implementation");
    let src = gore_as::cache::structure::decompile(f, &refs);
    assert!(
        src.contains(".Condition.BindUFunction(this, n\"CanToast\")"),
        "deferred FName was displaced while building Last().Condition:\n{src}"
    );
    assert!(
        !src.contains("\u{2}argint"),
        "argint sentinel remains:\n{src}"
    );
}

/// A CALL-by-id to the emitted one-parameter script global must not borrow the zero-argument
/// arity of an unrelated/name-colliding Binds entry. The regression dropped the Character operand
/// at all 57 corpus call sites and was first surfaced by the runtime compiler hook.
#[test]
fn get_gothic_ability_component_call_keeps_character_argument() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let f = funcs
        .iter()
        .find(|f| f.func.ends_with("AFirewallBoundaryChecker::ApplyDamage"))
        .expect("hotfix corpus contains firewall ApplyDamage");
    let src = gore_as::cache::structure::decompile(f, &refs);
    assert!(
        src.contains("GetGothicAbilitySystemComponent(this.m_TargetCharacter)"),
        "Character operand was dropped from CALL-by-id:\n{src}"
    );
    assert!(
        !src.contains("GetGothicAbilitySystemComponent()"),
        "zero-arg regression remains:\n{src}"
    );

    let needle = "GetGothicAbilitySystemComponent(";
    let mut total = 0usize;
    let mut zero = Vec::new();
    let mut owners = std::collections::HashSet::new();
    let mut modules = std::collections::HashSet::new();
    for function in &funcs {
        let src = gore_as::cache::structure::decompile(function, &refs);
        let count = src.matches(needle).count();
        if count == 0 {
            continue;
        }
        total += count;
        owners.insert(function.func.clone());
        modules.insert(
            function
                .func
                .rsplit_once('.')
                .map(|(module, _)| module)
                .unwrap_or(&function.func)
                .to_string(),
        );
        if src.contains("GetGothicAbilitySystemComponent()") {
            zero.push(function.func.clone());
        }
    }
    // `structure::decompile` sees 56 body call sites; emit-all's file census sees 57 because its
    // module render includes one additional declaration/textual site.
    assert_eq!(total, 56, "hotfix structured call-site census drifted");
    assert_eq!(owners.len(), 52, "hotfix function census drifted");
    assert_eq!(modules.len(), 31, "hotfix module census drifted");
    assert!(zero.is_empty(), "zero-arg calls remain in: {zero:#?}");
}

/// Runtime-compiler feedback lock: ownerless/name-only Binds arities must not trim the
/// owner-specific AActor cache signatures, and iterator references must keep their float32
/// pointee declaration. These were the complete diagnostics surfaced by the first production
/// compiler-hook run after the CALL-by-id fix.
#[test]
fn runtime_compiler_residue_recovers_generically_in_real_cache() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    let modules = gore_as::cache::model::parse_modules(&b).expect("modules");
    refs.set_class_hierarchy(
        modules
            .iter()
            .flat_map(|m| m.classes.iter())
            .map(|c| (c.name.clone(), c.super_class.clone().unwrap_or_default()))
            .collect(),
    );
    refs.set_class_fields(
        modules
            .iter()
            .flat_map(|m| m.classes.iter())
            .map(|c| {
                (
                    c.name.clone(),
                    c.fields
                        .iter()
                        .map(|f| (f.name.clone(), f.ty.base_name(&refs)))
                        .collect(),
                )
            })
            .collect(),
    );
    refs.add_method_names(
        modules
            .iter()
            .flat_map(|m| m.classes.iter())
            .flat_map(|c| c.methods.iter())
            .map(|f| f.name.clone()),
    );
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }

    let emit = |module: &str| {
        let module = modules
            .iter()
            .find(|m| m.name == module)
            .unwrap_or_else(|| panic!("hotfix corpus contains module {module}"));
        gore_as::cache::emit::emit_module(module, &refs)
    };
    let soul =
        emit("AI.AIAgent.Creature.Skeleton.SkeletonMage.Spells.SoulHarvest.SoulHarvest_Visual");
    assert!(
        soul.contains("this.GetComponentsByClass(local_4, local_12);"),
        "SoulHarvest out-array was trimmed:\n{soul}"
    );
    let barrier = emit("Gameplay.Environment.BattleBlockingBarrier.BattleBlockingBarrierManager");
    assert!(
        barrier.contains("local_8.GetComponentsByClass(local_10, local_6);"),
        "barrier out-array was trimmed:\n{barrier}"
    );
    let lightning = emit("GAS.Abilities.Spells.Visuals.Electric.LightningRayVisual");
    assert!(
        lightning.contains("this.m_Target.GetComponentsByClass(local_6, local_4);"),
        "lightning out-array was trimmed:\n{lightning}"
    );
    let status = emit("GAS.UGothicCharacterStatusComponent");
    assert!(
        status.contains("local_22 = local_2.GetComponent(local_20, NAME_None);"),
        "GetComponent class arg was trimmed:\n{status}"
    );
    assert!(
        !status.contains("local_2.GetComponent()"),
        "zero-arg GetComponent regression remains:\n{status}"
    );
    let combat = emit("AI.States.FightAI.CombatState.AICombatBehaviorTree");
    let start = combat
        .find("void NormalizeWeights()")
        .expect("NormalizeWeights");
    let normalize = &combat[start..];
    assert!(
        normalize.contains("float32 local_20 = 0.0f;"),
        "iterator pointee remains int:\n{normalize}"
    );
    assert!(
        normalize.contains("local_20 = local_14.Proceed();"),
        "iterator value flow drifted:\n{normalize}"
    );

    // Whole-hotfix census for the two arity policies. The object calls must retain the cache
    // signature, while the value/template defaults remain eligible for the safe lowering.
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let status_func = funcs
        .iter()
        .find(|f| {
            f.func
                .ends_with("UGothicCharacterStatusComponent::BeginPlay_Implementation")
        })
        .expect("status BeginPlay bytecode");
    let name_none_pushes: Vec<_> = disassemble(&status_func.bytecode)
        .expect("status disasm")
        .into_iter()
        .filter(|ins| {
            ins.op.name == "PshGPtr"
                && ins
                    .qwords
                    .first()
                    .and_then(|ptr| refs.global_by_ptr(*ptr as i64))
                    == Some("NAME_None")
        })
        .collect();
    assert_eq!(
        name_none_pushes.len(),
        1,
        "physical trailing-default operand census drifted"
    );
    assert_eq!(
        name_none_pushes[0].offset_dw, 54,
        "NAME_None must remain the deeper physical operand carried across StaticClass/TSubclassOf"
    );
    let mut counts = std::collections::BTreeMap::<(&str, &str), usize>::new();
    for f in &funcs {
        for ins in disassemble(&f.bytecode).expect("disasm") {
            if !matches!(ins.op.name, "CALLSYS" | "Thiscall1") {
                continue;
            }
            let ptr = ins.qwords.first().copied().unwrap_or(0) as i64;
            let Some(owner) = refs.func_owner_by_ptr(ptr) else {
                continue;
            };
            let Some(name) = refs.func_by_ptr(ptr) else {
                continue;
            };
            let cache_arity = refs.func_params_by_ptr(ptr).map(|p| p.len());
            if matches!(
                (owner, name, cache_arity),
                ("AActor", "GetComponent", Some(2))
                    | ("AActor", "GetComponentsByClass", Some(2))
                    | ("FVector", "Equals", Some(2))
                    | ("TArray", "Last", Some(1))
            ) {
                *counts.entry((owner, name)).or_default() += 1;
                let arity = refs.native_arity_by_ptr(ptr, name);
                match (owner, name) {
                    ("AActor", _) => assert_eq!(arity, None),
                    ("FVector", "Equals") => assert_eq!(arity, Some(1)),
                    ("TArray", "Last") => assert_eq!(arity, Some(0)),
                    _ => unreachable!(),
                }
            }
        }
    }
    assert_eq!(counts.get(&("AActor", "GetComponent")), Some(&1));
    assert_eq!(counts.get(&("AActor", "GetComponentsByClass")), Some(&3));
    assert_eq!(counts.get(&("FVector", "Equals")), Some(&1));
    assert_eq!(counts.get(&("TArray", "Last")), Some(&8_106));
}

/// Binds contains an unrelated two-arg `UTimelineComponent::AddEvent`, while the owner-known
/// `FPerceptionHandler::AddEvent` cache reference has one parameter. A name-only arity match
/// stole the next fluent-chain argument and shifted every later call.
#[test]
fn owner_known_native_arity_does_not_shift_real_fluent_chains() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let decomp = |needle: &str| {
        let f = funcs
            .iter()
            .find(|f| f.func.contains(needle))
            .unwrap_or_else(|| panic!("hotfix corpus contains {needle}"));
        gore_as::cache::structure::decompile(f, &refs)
    };

    let heard = decomp("GAS.PerceptionEventMixins::OnHeardInstantSoundEvent");
    assert!(
        heard.contains(
            ".AddEvent(GameplayTag::Event_Perception_Gained_Hear)\
             .OnCharactersWith(SoundTag).ViaSense(EPerceptionSense(local_1))\
             .RequireSensingSource((local_2 != 0))"
        ),
        "OnHeard fluent args shifted:\n{heard}"
    );
    assert!(!heard.contains("\u{2}argint"), "argint remains:\n{heard}");

    let bait = decomp("UAIState_Test_BaitWithFood::SetupPerceptions_Implementation");
    assert!(
        bait.contains(
            ".AddEvent(GameplayTag::Perception_Smell_Food)\
             .ViaSense(EPerceptionSense(local_1)).WithAwareness(70.0f)\
             .PerceptionGained.BindUFunction(this, n\"OnSmelledFood\")"
        ),
        "BaitWithFood fluent args shifted:\n{bait}"
    );
    assert!(!bait.contains("\u{2}argint"), "argint remains:\n{bait}");

    let force = decomp("AGornSleeper_ElectrifiedCell::ForceRemoveDamage");
    assert!(
        force.contains(
            ".RemoveActiveGameplayEffectBySourceEffect(local_18.m_HitEffect, nullptr, -1)"
        ),
        "GetDefaultObject borrowed the outer nullptr argument:\n{force}"
    );
    assert!(!force.contains("\u{2}argint"), "argint remains:\n{force}");

    let party = decomp(
        "URegionTrait_Story_MonasteryRuins_GornParty::HandleHandleOwnerEndPlay_Implementation",
    );
    assert!(
        party.contains(
            "local_20.GetKey().AbilitySystemComponent\
             .RemoveActiveGameplayEffect(local_20.GetValue(), -1)"
        ),
        "GetKey borrowed the outer effect-handle argument:\n{party}"
    );
    assert!(!party.contains("\u{2}argint"), "argint remains:\n{party}");
}

/// Real emitter lock for the strictly-typed PSF copy-constructor lowering. The function has
/// two independent instances: FTransform and FVector return temporaries copied into locals.
#[test]
fn same_type_psf_copy_constructor_writes_real_destination_locals() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }
    let modules = gore_as::cache::model::parse_modules(&b).expect("parse modules");
    let module = modules
        .iter()
        .find(|m| m.name == "GAS.GASCharacterStateMixins")
        .expect("hotfix GASCharacterStateMixins module");
    let src = gore_as::cache::emit::emit_module(module, &refs);
    let start = src
        .find("void TeleportToWaypointAndExchangeDailyRoutineToClass(")
        .expect("target function");
    let body = &src[start..];
    let end = body
        .find("\nvoid ExchangeDailyRoutineToClass(")
        .expect("next function boundary");
    let body = &body[..end];

    assert!(
        body.contains("local_28 = FTransform(local_52);"),
        "FTransform copy destination remains unwritten:\n{body}"
    );
    assert!(
        body.contains("local_136 = FVector(local_128);"),
        "FVector copy destination remains unwritten:\n{body}"
    );
    assert!(
        body.contains(
            "MagicScript::FindFloorAtLocation(local_128, local_118, local_54, \
             ECollisionChannel(local_121), -200.0f, 10.0f, (local_3 != 0))"
        ),
        "FindFloor arguments shifted:\n{body}"
    );
    assert!(!body.contains("\u{2}argint"), "argint remains:\n{body}");
}

/// Real-hotfix lock for a switch in a struct-RVO function that mixes ordinary
/// `break -> JOIN` paths with early `RVO store; cleanup; JMP shared RET` returns.
#[test]
fn mixed_rvo_switch_with_normal_join_recovers_in_real_cache() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    let binds = std::env::var_os("GORE_AS_BINDS")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::path::Path::new(&path)
                .parent()
                .map(|p| p.join("Binds.Cache"))
        });
    if let Some(api) = binds
        .as_deref()
        .and_then(gore_as::cache::binds::NativeApi::load)
    {
        refs.set_native_api(api);
    }
    let funcs = collect_function_bytecodes(&b).expect("collect");
    let f = funcs
        .iter()
        .find(|f| f.func.ends_with("::MakeNewCrimeRegisterData"))
        .expect("hotfix corpus contains MakeNewCrimeRegisterData");
    let src = gore_as::cache::structure::decompile(f, &refs);
    assert!(src.contains("switch ("), "switch was not recovered:\n{src}");
    assert!(!src.contains("// JMPP"), "JMPP marker remains:\n{src}");
    assert!(
        src.matches("return __return;").count() >= 3,
        "mixed RVO early-return edges were not retained:\n{src}"
    );

    // Negative integration gate: redirect the first case's copy destination away from the
    // hidden RVO slot while preserving instruction widths and the entire CFG. The early
    // JMP-to-RET then has no local out-slot proof, so switch recovery must fail atomically.
    let mut missing_store = f.clone();
    let dst_push = missing_store
        .bytecode
        .get_mut(180)
        .expect("hotfix RVO destination push at dword 180");
    let encoded = *dst_push as u32;
    assert_eq!((encoded >> 16) as u16, 65_534);
    *dst_push = ((encoded & 0xffff) | (65_532_u32 << 16)) as i32;
    let rejected = gore_as::cache::structure::decompile(&missing_store, &refs);
    assert!(
        rejected.contains("// JMPP"),
        "switch with an unproved RVO early return was accepted:\n{rejected}"
    );
}

#[test]
fn native_enum_member_register_pushes_recover_crime_relationships() {
    let Ok(path) = std::env::var("GORE_AS_REAL_CACHE") else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let b = std::fs::read(&path).expect("read real cache");
    let mut refs = RefResolver::build(&b).expect("resolver");
    let modules = gore_as::cache::model::parse_modules(&b).expect("modules");
    let hierarchy = modules
        .iter()
        .flat_map(|m| m.classes.iter())
        .map(|c| (c.name.clone(), c.super_class.clone().unwrap_or_default()))
        .collect();
    let fields = modules
        .iter()
        .flat_map(|m| m.classes.iter())
        .map(|c| {
            (
                c.name.clone(),
                c.fields
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.base_name(&refs)))
                    .collect(),
            )
        })
        .collect();
    refs.set_class_hierarchy(hierarchy);
    refs.set_class_fields(fields);
    if let Ok(path) = std::env::var("GORE_AS_BINDS") {
        if let Some(api) = gore_as::cache::binds::NativeApi::load(std::path::Path::new(&path)) {
            refs.set_native_api(api);
        }
    }
    let m = modules
        .iter()
        .find(|m| m.classes.iter().any(|c| c.name == "UCrimeSeverityModifier"))
        .expect("crime module");
    let src = gore_as::cache::emit::emit_module(m, &refs);
    let start = src
        .find("ERelationship GetRelationshipTowardsVictim")
        .expect("first target");
    let end = src[start..]
        .find("class UCrimeSeverityMultiplicativeModifier")
        .expect("next class");
    let targets = &src[start..start + end];
    assert!(
        !targets.contains("body not fully recovered"),
        "crime relationship functions remain stubbed:\n{targets}"
    );
    for member in [
        "local_20.RelationshipTowardsPerson",
        "local_34.RelationshipTowardsGuild",
        "local_20.RelativeRankTowardsPerson",
        "local_34.RelativeRankTowardsGuild",
    ] {
        assert!(
            targets.contains(&format!("local_4.Add({member});")),
            "native enum member did not reach TArray::Add:\n{targets}"
        );
    }

    // Negative integration gate: preserve the CFG and register-push shape, but redirect the
    // first relationship field reference (offset 8) to the adjacent relative-rank enum field
    // (offset 9). The now-proven enum mismatch must still force an argtype stub; the recovery
    // must not merely suppress type checking for LoadRObjR/PshRPtr arguments.
    let mut mismatched = m.clone();
    let method = mismatched
        .classes
        .iter_mut()
        .find(|c| c.name == "UCrimeSeverityModifier")
        .and_then(|c| {
            c.methods
                .iter_mut()
                .find(|f| f.name == "GetRelationshipTowardsVictim")
        })
        .expect("relationship method");
    let load = disassemble(&method.bytecode)
        .expect("disasm")
        .into_iter()
        .find(|ins| {
            ins.op.name == "LoadRObjR"
                && ins.words.get(1).copied() == Some(8)
                && ins
                    .dwords
                    .first()
                    .and_then(|id| refs.type_by_id(*id as i32))
                    == Some("FCrimeVictimPersonHandle")
        })
        .expect("person relationship member load");
    let operand = method
        .bytecode
        .get_mut(load.offset_dw + 1)
        .expect("LoadRObjR field-offset operand");
    let encoded = *operand as u32;
    assert_eq!((encoded & 0xffff) as u16, 8);
    *operand = ((encoded & 0xffff_0000) | 9) as i32;

    let rejected = gore_as::cache::emit::emit_module(&mismatched, &refs);
    let rejected = &rejected[rejected
        .find("ERelationship GetRelationshipTowardsVictim")
        .expect("mutated target")..];
    let rejected = &rejected[..rejected
        .find("ERelationshipRelativeRank GetRelativeRankTowardsVictim")
        .expect("next target")];
    assert!(
        rejected.contains("stub [argmismatch:argtype]"),
        "known mismatched enum was accepted:\n{rejected}"
    );
}
