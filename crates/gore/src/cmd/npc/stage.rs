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

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use gore_as::cache::faithfulness;
use serde::{Deserialize, Serialize};

use super::workspace::Manifest;

/// Der Stempel neben einem vorgehaltenen Quellbaum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeStamp {
    /// Die Cache, aus der der Baum emittiert wurde.
    pub cache_sha256: String,
    /// Wie viele Dateien geschrieben wurden — eine grobe Vollständigkeitsprobe.
    pub modules: usize,
}

/// Der Dateiname des Stempels im Baumverzeichnis.
pub const TREE_STAMP_NAME: &str = ".gore-npc-tree.json";

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

/// `compile-module` reads the resolved installation, so both it and the selected cache must
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
                "reading compiler base cache {}. `compile-module` requires a matching game installation",
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

/// Die Kommandos, die diese Arbeit übersetzen und verpacken.
///
/// `stage` führt sie nicht aus. Der Voll-Baum-Lauf dauert eine Viertelstunde, und ein Werkzeug,
/// das den ungefragt startet, nimmt dem Nutzer die Entscheidung ab, wann er wartet.
pub fn build_commands(
    manifest: &Manifest,
    dir: &str,
    tree: &str,
    mod_name: &str,
    game: Option<&str>,
) -> Vec<String> {
    let game_arg = match game {
        Some(path) => format!(" --game \"{path}\""),
        None => String::new(),
    };
    // Der Preflight lehnt einen Arbeitsordner unterhalb des Ausgabe-Elternverzeichnisses ab, also
    // liegt er bewusst daneben statt darin.
    let work = format!("{dir}.work");
    let mut out = Vec::new();
    match route_of(manifest) {
        Route::FullTree => {
            out.push(format!(
                "gore as compile \"{tree}\" -o \"{dir}/full.Cache\" \
                 --mini \"{dir}/{mod_name}.mini.Cache\" --work-dir \"{work}\" \
                 --backend standalone{game_arg}"
            ));
        }
        Route::SingleModule => {
            let edit = manifest
                .level_edit()
                .expect("a checkout or suppression always edits a shipped module");
            out.push(format!(
                "gore as compile-module --backend standalone --op edit \
                 --module \"{}\" --rel-path \"{}\" --source \"{dir}/{}\" \
                 --work-dir \"{work}\" -o \"{dir}/{mod_name}.mini.Cache\"{game_arg}",
                edit.module, edit.relative_path, edit.source_file
            ));
        }
    }
    out.push(format!(
        "gore mod build --spec \"{dir}/spec.json\" -o \"{dir}/build\""
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::npc::workspace::{ModuleEdit, Operation};

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
        assert!(commands[0].starts_with("gore as compile \"tree\""));
        assert!(commands[0].contains("--mini \"ws/MyMod.mini.Cache\""));
        assert!(commands[0].contains("--backend standalone"));
        assert!(commands[0].contains("--game \"G\""));
    }

    #[test]
    fn the_single_module_route_names_the_module_and_its_source() {
        let commands = build_commands(&suppression(), "ws", "tree", "MyMod", None);
        assert!(commands[0].starts_with("gore as compile-module"));
        assert!(commands[0].contains("--module \"LevelScripts.XardasTower_AI\""));
        assert!(commands[0].contains("--rel-path \"LevelScripts/XardasTower_AI.as\""));
        assert!(commands[0].contains("--source \"ws/XardasTower_AI.as\""));
        assert!(!commands[0].contains("--game"));
    }

    #[test]
    fn the_work_directory_never_sits_under_the_output_parent() {
        // Sonst bricht der Preflight des Compilers ab: work-dir und Ausgabe-Elternverzeichnis
        // müssen disjunkt sein.
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert!(commands[0].contains("--work-dir \"ws.work\""));
            assert!(!commands[0].contains("--work-dir \"ws/"));
        }
    }

    #[test]
    fn both_routes_end_with_the_bundle_build() {
        for manifest in [authored(), suppression()] {
            let commands = build_commands(&manifest, "ws", "tree", "MyMod", None);
            assert_eq!(commands.len(), 2);
            assert!(commands[1].starts_with("gore mod build --spec \"ws/spec.json\""));
        }
    }

    #[test]
    fn a_tree_stamp_round_trips() {
        let stamp = TreeStamp {
            cache_sha256: "abc".to_string(),
            modules: 7317,
        };
        let json = serde_json::to_string(&stamp).expect("serialize");
        assert_eq!(
            serde_json::from_str::<TreeStamp>(&json).expect("deserialize"),
            stamp
        );
    }
}
