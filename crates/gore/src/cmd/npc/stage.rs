//! Das verfasste Arbeitsverzeichnis für den Compiler herrichten.
//!
//! Neue Figuren werden als unabhängiger Snapshot ihrer verfassten Module vorbereitet.
//! `gore as compile --overlays --mini` übernimmt alle übrigen Module aus der exakten Basis;
//! ein vollständiger Quellbaum wird weder exportiert noch erneut emittiert.
//! Checkouts und Unterdrückungen behalten den Einzelmodulweg `gore as compile-module`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use gore_as::cache::faithfulness;
use sha2::{Digest, Sha256};

use super::workspace::Manifest;

pub const STAGED_SOURCE_NAME: &str = ".gore-npc-staged-source.as";

/// Welchen Weg diese Arbeit nimmt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Neue oder mehrere Module: `gore as compile --overlays --mini`.
    Overlays,
    /// Nur ein ausgeliefertes Modul: `gore as compile-module`.
    SingleModule,
}

/// Der Weg, den diese Arbeit verlangt.
///
/// A new character adds a module called by an existing one. The sparse input carries both;
/// the compiler retains every unchanged dependency from the sealed base cache.
/// Checkouts and suppressions edit one module. Both compiler routes apply the same
/// default-target preservation proof.
pub fn route_of(manifest: &Manifest) -> Route {
    if manifest.authored_module().is_some() || manifest.modules.len() > 1 {
        Route::Overlays
    } else {
        Route::SingleModule
    }
}

/// Both compiler routes read the resolved installation. Its cache and the selected cache must
/// match the workspace base before staging can print a command for that installation.
pub fn compiler_game_for(
    manifest: &Manifest,
    cache: &Path,
    game: Option<PathBuf>,
) -> Result<PathBuf> {
    let root = gore_loc::config::game_root(game).context("resolving compiler game path")?;
    let script_cache = gore_mod::pristine_script_cache_source(&root)
        .context("selecting the pristine compiler base cache")?;
    for path in [cache, script_cache.path.as_path()] {
        let bytes = fs::read(path).with_context(|| {
            format!(
                "reading compiler base cache {}. Compilation requires a matching game installation",
                path.display()
            )
        })?;
        let digest: String = faithfulness::cache_seal(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if digest != manifest.cache_sha256 {
            bail!(
                "the cache at {} is not the cache this workspace was authored against. \
                 `stage` cannot print a safe compile command for a different or arbitrary \
                 --cache file; pass --game pointing to a matching installation",
                path.display()
            );
        }
    }
    Ok(root)
}

/// Der Bundle-Spec-Eintrag für diese Arbeit.
///
/// `op` ist immer `edit`: die Mini ersetzt ein ausgeliefertes Modul, auch wenn sie daneben ein
/// neues trägt. Beim Deploy werden vorhandene Module ersetzt und neue angehängt, als eine Einheit.
pub fn spec_json(manifest: &Manifest, mod_name: &str) -> serde_json::Value {
    serde_json::json!({
        "meta": { "name": mod_name, "version": "0.1.0", "author": "" },
        "scripts": [{
            "op": "edit",
            "module_name": manifest.level_module,
            "mini_cache": format!("{mod_name}.mini.Cache"),
        }],
    })
}

/// Quote a literal argument for the platform's interactive shell (PowerShell or POSIX sh).
pub fn shell_quote(value: &str) -> String {
    #[cfg(windows)]
    let escaped = value.replace('\'', "''");
    #[cfg(not(windows))]
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}

pub fn work_dir(dir: &Path) -> PathBuf {
    let mut path = dir.components().collect::<PathBuf>().into_os_string();
    path.push(".work");
    PathBuf::from(path)
}

/// Die Kommandos, die diese Arbeit übersetzen und verpacken.
///
/// `stage` führt sie nicht aus. Jeder Befehl bindet die Basis und die geprüften Quelldateien
/// über ihre exakten Hashes; spätere Änderungen am Snapshot werden beim Kompilieren abgelehnt.
pub fn build_commands(
    manifest: &Manifest,
    checked_sources: &BTreeMap<String, String>,
    dir: &str,
    source_root: &str,
    mod_name: &str,
    game: Option<&str>,
) -> Result<Vec<String>> {
    let game_arg = match game {
        Some(path) => format!(" --game {}", shell_quote(path)),
        None => String::new(),
    };
    // Der Preflight lehnt einen Arbeitsordner unterhalb des Ausgabe-Elternverzeichnisses ab, also
    // liegt er bewusst daneben statt darin.
    let work = work_dir(Path::new(dir));
    let work_arg = shell_quote(&work.display().to_string());
    let mini_arg = shell_quote(
        &Path::new(dir)
            .join(format!("{mod_name}.mini.Cache"))
            .display()
            .to_string(),
    );
    let base_arg = shell_quote(&manifest.cache_sha256);
    let mut out = Vec::new();
    match route_of(manifest) {
        Route::Overlays => {
            let only_changes = manifest
                .modules
                .iter()
                .map(|edit| {
                    let source = checked_sources.get(&edit.source_file).with_context(|| {
                        format!("the validated NPC source snapshot is missing {}", edit.source_file)
                    })?;
                    let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
                    Ok::<_, anyhow::Error>(format!(
                        " --only-change {}",
                        shell_quote(&format!("{}:{}:{}:{digest}", edit.op, edit.module, edit.relative_path))
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .concat();
            out.push(format!(
                "gore as compile {} --overlays -o {} --mini {mini_arg} --work-dir {work_arg} \
                 --backend standalone --expect-base-sha256 {base_arg}{only_changes}{game_arg}",
                shell_quote(source_root),
                shell_quote(&Path::new(dir).join("full.Cache").display().to_string()),
            ));
        }
        Route::SingleModule => {
            let edit = manifest
                .level_edit()
                .expect("a checkout or suppression always edits a shipped module");
            let source = checked_sources.get(&edit.source_file).with_context(|| {
                format!("the validated NPC source snapshot is missing {}", edit.source_file)
            })?;
            let source_digest = shell_quote(&format!("{:x}", Sha256::digest(source.as_bytes())));
            out.push(format!(
                "gore as compile-module --backend standalone --op edit \
                 --module {} --rel-path {} --source {} \
                 --work-dir {work_arg} -o {mini_arg} --expect-base-sha256 {base_arg} \
                 --expect-source-sha256 {source_digest}{game_arg}",
                shell_quote(&edit.module),
                shell_quote(&edit.relative_path),
                shell_quote(&Path::new(dir).join(STAGED_SOURCE_NAME).display().to_string()),
            ));
        }
    }
    out.push(format!(
        "gore mod build --spec {} -o {}",
        shell_quote(&Path::new(dir).join("spec.json").display().to_string()),
        shell_quote(&Path::new(dir).join("build").display().to_string()),
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::npc::workspace::{ModuleEdit, Operation};

    fn build_commands(
        manifest: &Manifest,
        dir: &str,
        source_root: &str,
        mod_name: &str,
        game: Option<&str>,
    ) -> Vec<String> {
        let sources = manifest
            .modules
            .iter()
            .map(|edit| (edit.source_file.clone(), "class Test {}".to_string()))
            .collect();
        super::build_commands(manifest, &sources, dir, source_root, mod_name, game).unwrap()
    }

    fn quoted_child(dir: &str, name: &str) -> String {
        shell_quote(&Path::new(dir).join(name).display().to_string())
    }

    fn level_edit() -> ModuleEdit {
        ModuleEdit {
            module: "LevelScripts.XardasTower_AI".to_string(),
            relative_path: "LevelScripts/XardasTower_AI.as".to_string(),
            source_file: "XardasTower_AI.as".to_string(),
            pristine_file: Some("pristine/XardasTower_AI.as".to_string()),
            op: "edit".to_string(),
        }
    }

    fn authored() -> Manifest {
        Manifest {
            operation: Operation::New,
            npc_id: "MINE".to_string(),
            derived_from: Some("OC_STT_Diego".to_string()),
            modules: vec![
                ModuleEdit {
                    module: "AI.AIAgent.Human.Config.MINE.MINE".to_string(),
                    relative_path: "AI/AIAgent/Human/Config/MINE/MINE.as".to_string(),
                    source_file: "MINE.as".to_string(),
                    pristine_file: None,
                    op: "add".to_string(),
                },
                level_edit(),
            ],
            world_points: vec!["UWP_A".to_string()],
            level_module: "LevelScripts.XardasTower_AI".to_string(),
            cache_sha256: "abc".to_string(),
            modular_visuals: false,
        }
    }

    fn suppression() -> Manifest {
        Manifest {
            operation: Operation::Suppress,
            npc_id: "OC_STT_Diego".to_string(),
            derived_from: None,
            modules: vec![level_edit()],
            world_points: vec!["UWP_A".to_string()],
            level_module: "LevelScripts.XardasTower_AI".to_string(),
            cache_sha256: "abc".to_string(),
            modular_visuals: false,
        }
    }

    #[test]
    fn a_new_character_takes_the_sparse_overlay_route() {
        assert_eq!(route_of(&authored()), Route::Overlays);
    }

    #[test]
    fn a_suppression_takes_the_single_module_route() {
        // Only the existing level module changes.
        assert_eq!(route_of(&suppression()), Route::SingleModule);
    }

    #[test]
    fn a_checkout_uses_the_single_module_route_and_command() {
        let mut manifest = suppression();
        manifest.operation = Operation::Checkout;
        assert_eq!(route_of(&manifest), Route::SingleModule);
        let commands = build_commands(&manifest, "work/diego", "unused", "ToughDiego", None);
        assert!(commands[0].contains("compile-module --backend standalone --op edit"));
        assert!(!commands[0].contains("unused"));
    }

    #[test]
    fn the_spec_entry_edits_the_level_module_in_both_routes() {
        for manifest in [authored(), suppression()] {
            let spec = spec_json(&manifest, "MyMod");
            assert_eq!(spec["scripts"][0]["op"], "edit");
            assert_eq!(
                spec["scripts"][0]["module_name"],
                "LevelScripts.XardasTower_AI"
            );
            assert_eq!(spec["scripts"][0]["mini_cache"], "MyMod.mini.Cache");
            assert_eq!(spec["meta"]["name"], "MyMod");
        }
    }

    #[test]
    fn the_sparse_route_asks_for_a_multi_module_mini() {
        let commands = build_commands(&authored(), "ws", "tree", "MyMod", Some("G"));
        assert!(commands[0].starts_with("gore as compile 'tree' --overlays"));
        assert!(commands[0].contains(&format!(
            "--mini {}",
            quoted_child("ws", "MyMod.mini.Cache")
        )));
        assert!(commands[0].contains("--backend standalone"));
        assert!(commands[0].contains("--only-change 'add:AI.AIAgent.Human.Config.MINE.MINE:AI/AIAgent/Human/Config/MINE/MINE.as:"));
        assert!(commands[0].contains("--only-change 'edit:LevelScripts.XardasTower_AI:LevelScripts/XardasTower_AI.as:"));
        let digest = format!("{:x}", Sha256::digest(b"class Test {}"));
        assert_eq!(commands[0].matches(&format!(":{digest}'")).count(), 2);
        assert!(commands[0].contains("--game 'G'"));
    }

    #[test]
    fn the_single_module_route_names_the_module_and_its_source() {
        let commands = build_commands(&suppression(), "ws", "tree", "MyMod", None);
        assert!(commands[0].starts_with("gore as compile-module"));
        assert!(commands[0].contains("--module 'LevelScripts.XardasTower_AI'"));
        assert!(commands[0].contains("--rel-path 'LevelScripts/XardasTower_AI.as'"));
        assert!(commands[0].contains(&format!(
            "--source {}",
            quoted_child("ws", STAGED_SOURCE_NAME)
        )));
        let digest = format!("{:x}", Sha256::digest(b"class Test {}"));
        assert!(commands[0].contains(&format!(
            "--expect-source-sha256 '{digest}'"
        )));
        assert!(!commands[0].contains("--game"));
    }

    #[test]
    fn both_compile_routes_bind_to_the_checked_cache() {
        for mut manifest in [authored(), suppression()] {
            manifest.cache_sha256 = "a".repeat(64);
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert!(commands[0].contains(&format!(
                "--expect-base-sha256 '{}'", manifest.cache_sha256
            )));
        }
    }

    #[test]
    fn the_work_directory_never_sits_under_the_output_parent() {
        // Sonst bricht der Preflight des Compilers ab: work-dir und Ausgabe-Elternverzeichnis
        // müssen disjunkt sein.
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert!(commands[0].contains("--work-dir 'ws.work'"));
            assert!(!commands[0].contains("--work-dir 'ws/"));
        }
    }

    #[test]
    fn trailing_workspace_separator_keeps_work_directory_beside_outputs() {
        assert_eq!(work_dir(Path::new("ws/")), PathBuf::from("ws.work"));
        #[cfg(windows)]
        assert_eq!(work_dir(Path::new("ws\\")), PathBuf::from("ws.work"));
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws/", "tree", "MyMod", None);
            assert!(commands[0].contains("--work-dir 'ws.work'"));
            assert!(!commands[0].contains("--work-dir 'ws/.work'"));
        }
    }

    #[test]
    fn canonical_current_workspace_puts_work_beside_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("npc");
        fs::create_dir(&workspace).unwrap();
        let resolved = fs::canonicalize(workspace.join(".")).unwrap();
        let work = work_dir(&resolved);
        assert_eq!(work, resolved.with_file_name("npc.work"));
        assert!(!work.starts_with(&resolved));
        let commands = build_commands(
            &authored(),
            &resolved.display().to_string(),
            "tree",
            "MyMod",
            None,
        );
        assert!(commands[0].contains(&format!(
            "--work-dir {}",
            shell_quote(&work.display().to_string())
        )));
        assert!(commands[0].contains(&format!(
            "-o {}",
            shell_quote(&resolved.join("full.Cache").display().to_string())
        )));
        assert!(commands[1].contains(&format!(
            "--spec {}",
            shell_quote(&resolved.join("spec.json").display().to_string())
        )));
    }

    #[test]
    fn both_routes_end_with_the_bundle_build() {
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert_eq!(commands.len(), 2);
            assert!(commands[1].starts_with(&format!(
                "gore mod build --spec {}",
                quoted_child("ws", "spec.json")
            )));
        }
    }

    #[test]
    fn printed_commands_keep_shell_metacharacters_in_literal_arguments() {
        let dir = "C:/mods/$HOME`bad'$(echo bad)";
        let tree = "C:/tree/$HOME`bad'$(echo bad)";
        #[cfg(windows)]
        assert_eq!(shell_quote(dir), "'C:/mods/$HOME`bad''$(echo bad)'");
        #[cfg(not(windows))]
        assert_eq!(shell_quote(dir), "'C:/mods/$HOME`bad'\\''$(echo bad)'");
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, dir, tree, "MyMod", Some(dir));
            assert!(commands[0].contains(&format!("--game {}", shell_quote(dir))));
            assert!(commands[0].contains(&format!(
                "--work-dir {}",
                shell_quote(&work_dir(Path::new(dir)).display().to_string())
            )));
            assert!(commands[1].contains(&format!(
                "--spec {}",
                quoted_child(dir, "spec.json")
            )));
        }
        let commands = build_commands(&authored(), dir, tree, "MyMod", Some(dir));
        assert!(commands[0].contains(&shell_quote(tree)));
        let commands = build_commands(&suppression(), dir, tree, "MyMod", Some(dir));
        assert!(commands[0].contains(&format!(
            "--source {}",
            quoted_child(dir, STAGED_SOURCE_NAME)
        )));
    }
}
