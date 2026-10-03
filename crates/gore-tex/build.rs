fn main() {
    // intel_tex_2 ships C++ objects. Its Linux archive uses exceptions, so
    // executables and cdylibs consuming the texture library need the runtime.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
