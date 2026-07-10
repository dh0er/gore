use gore_as::cache::emit_all::rename_free_fn;

#[test]
fn renames_decl_and_free_calls_only() {
    let src = "void Foo(){} void bar(){ Foo(); obj.Foo(); A::Foo(); }";
    let out = rename_free_fn(src, "Foo", "Foo_g3");
    assert!(out.contains("void Foo_g3(){}"));
    assert!(out.contains("Foo_g3();")); // free call renamed
    assert!(out.contains("obj.Foo();")); // member call untouched
    assert!(out.contains("A::Foo();")); // scoped call untouched
}
