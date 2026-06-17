use std::path::Path;

fn main() {
    let vendor = Path::new("vendor/ooz");
    let sources = [
        "bitknit.cpp",
        "compr_entropy.cpp",
        "compr_kraken.cpp",
        "compr_leviathan.cpp",
        "compr_match_finder.cpp",
        "compr_mermaid.cpp",
        "compr_multiarray.cpp",
        "compr_tans.cpp",
        "compress.cpp",
        "kraken.cpp",
        "lzna.cpp",
    ];

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .define("OOZ_BUILD_DLL", "1") // drops the CLI main() in kraken.cpp
        .include(vendor)
        .include(vendor.join("simde"))
        .file("csrc/ooz_shim.cpp");
    for src in sources {
        build.file(vendor.join(src));
    }
    if build.get_compiler().is_like_msvc() {
        build.flag("/wd4267").flag("/wd4334").flag("/wd4244");
    } else {
        build.flag_if_supported("-w");
    }
    build.compile("goresave_ooz");

    println!("cargo:rerun-if-changed=csrc/ooz_shim.cpp");
    println!("cargo:rerun-if-changed=vendor/ooz");
}
