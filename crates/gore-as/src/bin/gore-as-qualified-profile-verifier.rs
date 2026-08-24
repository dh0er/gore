use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{bail, Context as _};
use gore_as::compiler_profile::capture::verify_qualified_profile_package_v1;
use serde::Serialize;

const VERIFICATION_SCHEMA_V1: &str = "gore.as.qualified-profile-verification";

#[derive(Serialize)]
struct VerificationResultV1 {
    schema: &'static str,
    schema_version: u32,
    qualified: bool,
    profile_sha256: String,
    manifest_sha256: String,
    promotion_receipt_sha256: String,
    tree_sha256: String,
    file_count: u32,
}

fn one_profile_root() -> anyhow::Result<PathBuf> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let root = arguments.next().ok_or_else(|| {
        anyhow::anyhow!("usage: gore-as-qualified-profile-verifier <absolute-profile-root>")
    })?;
    if arguments.next().is_some() || root == OsString::new() {
        bail!("usage: gore-as-qualified-profile-verifier <absolute-profile-root>");
    }
    Ok(PathBuf::from(root))
}

fn main() -> anyhow::Result<()> {
    let root = one_profile_root()?;
    let verification = verify_qualified_profile_package_v1(&root)
        .with_context(|| format!("qualified profile verification failed for {root:?}"))?;
    let result = VerificationResultV1 {
        schema: VERIFICATION_SCHEMA_V1,
        schema_version: 1,
        qualified: true,
        profile_sha256: verification.profile().profile_sha256.to_string(),
        manifest_sha256: verification.manifest_sha256().to_string(),
        promotion_receipt_sha256: verification.promotion_receipt_sha256().to_string(),
        tree_sha256: verification.tree_sha256().to_string(),
        file_count: verification.file_count(),
    };
    serde_json::to_writer(std::io::stdout().lock(), &result)
        .context("cannot write qualified profile verification result")?;
    println!();
    Ok(())
}
