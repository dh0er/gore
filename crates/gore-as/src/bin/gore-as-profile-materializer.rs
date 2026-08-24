use std::path::PathBuf;

use gore_as::compiler_profile::capture::materialize_unqualified_profile_package_from_paths_v1;

fn main() {
    if let Err(error) = run() {
        eprintln!("profile materialization failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let capture = PathBuf::from(arguments.next().ok_or(
        "usage: gore-as-profile-materializer <sealed.capture> <static-support.json> \
         <static-support-root> <new-output-root>",
    )?);
    let support_manifest = PathBuf::from(arguments.next().ok_or(
        "usage: gore-as-profile-materializer <sealed.capture> <static-support.json> \
         <static-support-root> <new-output-root>",
    )?);
    let support_root = PathBuf::from(arguments.next().ok_or(
        "usage: gore-as-profile-materializer <sealed.capture> <static-support.json> \
         <static-support-root> <new-output-root>",
    )?);
    let output_root = PathBuf::from(arguments.next().ok_or(
        "usage: gore-as-profile-materializer <sealed.capture> <static-support.json> \
         <static-support-root> <new-output-root>",
    )?);
    if arguments.next().is_some() {
        return Err(
            "usage: gore-as-profile-materializer <sealed.capture> <static-support.json> \
             <static-support-root> <new-output-root>"
                .into(),
        );
    }
    let result = materialize_unqualified_profile_package_from_paths_v1(
        &capture,
        &support_manifest,
        &support_root,
        &output_root,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
