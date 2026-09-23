//! Das verfasste Arbeitsverzeichnis für den Compiler herrichten.
//!
//! Zwei Geschwindigkeitsklassen, und die Wahl trifft nicht der Nutzer, sondern die Form der
//! Arbeit. Eine neue Figur berührt zwei Module — ein neues und ein ausgeliefertes — und braucht
//! deshalb den Voll-Baum-Weg über `gore as compile --mini`. Eine Unterdrückung berührt genau ein
//! ausgeliefertes Modul und läuft über `gore as compile-module`, das um ein Vielfaches schneller
//! ist.
//!
//! Der Voll-Baum kostet einmal rund 19 Minuten. Er wird deshalb neben dem Arbeitsverzeichnis
//! vorgehalten und an der Cache-Kennung wiedererkannt, statt bei jedem Lauf neu zu entstehen.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use gore_as::cache::faithfulness;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::workspace::Manifest;

/// Der Stempel neben einem vorgehaltenen Quellbaum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeStamp {
    /// Legacy trees may already contain overlays from earlier staging runs.
    #[serde(default)]
    pub format_version: u32,
    /// Die Cache, aus der der Baum emittiert wurde.
    pub cache_sha256: String,
    /// Wie viele Dateien geschrieben wurden — eine grobe Vollständigkeitsprobe.
    pub modules: usize,
    /// SHA-256 over every relative path and file body except this stamp.
    #[serde(default)]
    pub tree_sha256: String,
}

/// Der Dateiname des Stempels im Baumverzeichnis.
pub const TREE_STAMP_NAME: &str = ".gore-npc-tree.json";
pub const TREE_STAMP_VERSION: u32 = 3;
pub const STAGED_SOURCE_NAME: &str = ".gore-npc-staged-source.as";

/// Detect accidental edits to a reusable tree. This digest and its stamp are both local; the
/// stage-generated compile command checks the scoped source bytes against the sealed cache.
pub fn tree_sha256(root: &Path) -> Result<String> {
    fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                bail!(
                    "the reusable source tree contains a link: {}",
                    path.display()
                );
            }
            if metadata.is_dir() {
                collect(root, &path, files)?;
            } else if metadata.is_file()
                && path.file_name().and_then(|name| name.to_str()) != Some(TREE_STAMP_NAME)
            {
                let relative = path
                    .strip_prefix(root)
                    .expect("walked path stays under its root")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((relative, path));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = Sha256::new();
    for (relative, path) in files {
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        hash.update(&(relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Welchen Weg diese Arbeit nimmt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Ein neues und ein ausgeliefertes Modul: Voll-Baum, `gore as compile --mini`.
    FullTree,
    /// Nur ein ausgeliefertes Modul: `gore as compile-module`.
    SingleModule,
}

/// Der Weg, den diese Arbeit verlangt.
///
/// A new character adds a module called by an existing one, requiring the full graph.
/// Checkouts and suppressions edit one module. Both compiler routes apply the same
/// default-target preservation proof.
pub fn route_of(manifest: &Manifest) -> Route {
    if manifest.authored_module().is_some() {
        Route::FullTree
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
    let script_cache = gore_mod::resolve_game_paths(&root).script_cache;
    for path in [cache, script_cache.as_path()] {
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
/// `stage` führt sie nicht aus. Der Voll-Baum-Lauf dauert eine Viertelstunde, und ein Werkzeug,
/// das den ungefragt startet, nimmt dem Nutzer die Entscheidung ab, wann er wartet.
pub fn build_commands(
    manifest: &Manifest,
    checked_sources: &BTreeMap<String, String>,
    dir: &str,
    tree: &str,
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
    let mini_arg = shell_quote(&format!("{dir}/{mod_name}.mini.Cache"));
    let base_arg = shell_quote(&manifest.cache_sha256);
    let mut out = Vec::new();
    match route_of(manifest) {
        Route::FullTree => {
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
                "gore as compile {} -o {} --mini {mini_arg} --work-dir {work_arg} \
                 --backend standalone --expect-base-sha256 {base_arg}{only_changes}{game_arg}",
                shell_quote(tree),
                shell_quote(&format!("{dir}/full.Cache")),
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
                shell_quote(&format!("{dir}/{STAGED_SOURCE_NAME}")),
            ));
        }
    }
    out.push(format!(
        "gore mod build --spec {} -o {}",
        shell_quote(&format!("{dir}/spec.json")),
        shell_quote(&format!("{dir}/build")),
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
        tree: &str,
        mod_name: &str,
        game: Option<&str>,
    ) -> Vec<String> {
        let sources = manifest
            .modules
            .iter()
            .map(|edit| (edit.source_file.clone(), "class Test {}".to_string()))
            .collect();
        super::build_commands(manifest, &sources, dir, tree, mod_name, game).unwrap()
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
    fn a_new_character_takes_the_full_tree_route() {
        assert_eq!(route_of(&authored()), Route::FullTree);
    }

    #[test]
    fn a_suppression_takes_the_single_module_route() {
        // Sie berührt genau ein ausgeliefertes Modul. Sie über den Voll-Baum zu schicken hiesse,
        // eine Viertelstunde für eine entfernte Zeile zu warten.
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
    fn the_full_tree_route_asks_for_a_multi_module_mini() {
        let commands = build_commands(&authored(), "ws", "tree", "MyMod", Some("G"));
        assert!(commands[0].starts_with("gore as compile 'tree'"));
        assert!(commands[0].contains("--mini 'ws/MyMod.mini.Cache'"));
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
        assert!(commands[0].contains("--source 'ws/.gore-npc-staged-source.as'"));
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
    }

    #[test]
    fn both_routes_end_with_the_bundle_build() {
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert_eq!(commands.len(), 2);
            assert!(commands[1].starts_with("gore mod build --spec 'ws/spec.json'"));
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
                shell_quote(&format!("{dir}/spec.json"))
            )));
        }
        let commands = build_commands(&authored(), dir, tree, "MyMod", Some(dir));
        assert!(commands[0].contains(&shell_quote(tree)));
        let commands = build_commands(&suppression(), dir, tree, "MyMod", Some(dir));
        assert!(commands[0].contains(&format!(
            "--source {}",
            shell_quote(&format!("{dir}/{STAGED_SOURCE_NAME}"))
        )));
    }

    #[test]
    fn a_tree_stamp_round_trips() {
        let stamp = TreeStamp {
            format_version: TREE_STAMP_VERSION,
            cache_sha256: "abc".to_string(),
            modules: 7317,
            tree_sha256: "def".to_string(),
        };
        let json = serde_json::to_string(&stamp).expect("serialize");
        assert_eq!(
            serde_json::from_str::<TreeStamp>(&json).expect("deserialize"),
            stamp
        );
    }

    #[test]
    fn the_tree_digest_changes_with_paths_or_contents_and_ignores_its_stamp() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("nested")).unwrap();
        fs::write(tmp.path().join("nested/A.as"), "one").unwrap();
        let original = tree_sha256(tmp.path()).unwrap();
        fs::write(tmp.path().join(TREE_STAMP_NAME), "metadata").unwrap();
        assert_eq!(tree_sha256(tmp.path()).unwrap(), original);
        fs::write(tmp.path().join("nested/A.as"), "two").unwrap();
        assert_ne!(tree_sha256(tmp.path()).unwrap(), original);
    }
}
