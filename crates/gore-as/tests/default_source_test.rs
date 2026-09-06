//! Module-level behaviour of class-default recovery, without a game or a real cache.
//!
//! The statement-level rules live in `cache::default_source`'s unit tests. What matters here is
//! the property the recompile path depends on: a module either authors every one of its class
//! defaults or none of them, and the "none" case says why.

use gore_as::cache::emit::{emit_module, emit_module_with};
use gore_as::cache::model::{Class, Func, Module};
use gore_as::cache::refs::RefResolver;
use gore_as::cache::types::DataType;

fn void_type() -> DataType {
    DataType {
        token: 0x52,
        ..DataType::default()
    }
}

fn init_defaults(bytecode: Vec<i32>) -> Func {
    Func {
        name: "__InitDefaults".into(),
        namespace: String::new(),
        param_defaults: Vec::new(),
        ret: void_type(),
        params: Vec::new(),
        bytecode,
        variable_space: 0,
        obj_locals: Vec::new(),
        is_ufunction: false,
        traits: 0,
    }
}

fn class_with(name: &str, methods: Vec<Func>) -> Class {
    Class {
        name: name.into(),
        namespace: String::new(),
        super_class: None,
        fields: Vec::new(),
        methods,
        ctors: Vec::new(),
        flags: 0,
    }
}

fn module(classes: Vec<Class>) -> Module {
    Module {
        name: "Test.Module".into(),
        file: "Test/Module.as".into(),
        functions: Vec::new(),
        classes,
        enums: Vec::new(),
        globals: Vec::new(),
    }
}

#[test]
fn an_initializer_over_the_recovery_bound_suppresses_the_whole_module_and_says_so() {
    // Far past the bound; the size gate answers before anything tries to disassemble it.
    let huge = init_defaults(vec![0; 2_000_000]);
    let target = module(vec![
        class_with("USmall", Vec::new()),
        class_with("UHuge", vec![huge]),
    ]);
    let source = emit_module_with(&target, &RefResolver::default(), true);

    assert!(
        source.contains("class defaults are not authored in this module: UHuge"),
        "the module has to name the class that blocked recovery:\n{source}"
    );
    assert!(
        source.contains("recovery bound"),
        "the note has to give the reason:\n{source}"
    );
    assert!(
        source.contains("carried over byte-exact"),
        "the note has to say the defaults are not lost:\n{source}"
    );
    assert!(
        !source.contains("\n    default "),
        "a suppressed module must author no defaults at all:\n{source}"
    );
}

#[test]
fn a_module_without_initializers_gets_no_note() {
    let source = emit_module_with(
        &module(vec![class_with("UPlain", Vec::new())]),
        &RefResolver::default(),
        true,
    );
    assert!(
        !source.contains("class defaults are not authored"),
        "{source}"
    );
    assert!(!source.contains("\n    default "), "{source}");
}

#[test]
fn the_plain_entry_point_writes_no_defaults_and_no_note() {
    // Everything that hashes emitted source or feeds it back to the compiler goes through
    // `emit_module`, so it must keep the historical shape whatever the recovery would have said.
    let huge = init_defaults(vec![0; 200_000]);
    let target = module(vec![class_with("UHuge", vec![huge])]);
    let source = emit_module(&target, &RefResolver::default());
    assert!(!source.contains("class defaults are not authored"), "{source}");
    assert!(!source.contains("\n    default "), "{source}");
    assert_eq!(
        source,
        emit_module_with(&target, &RefResolver::default(), false),
        "the plain entry point is the defaults-off one"
    );
}
