//! Workspace editing for the bounded, generated daily-routine block.

use super::{
    edit, generate, read_manifest,
    routine_plan::{self, Activity, Phase, Plan},
    routine_spots::{needs_interaction_evidence, SpotCatalog},
    workspace::{Manifest, Operation},
};
use anyhow::{ensure, Context, Result};
use clap::Subcommand;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::{Component, Path, PathBuf},
};

const BEGIN: &str = "// gore npc routine begin v1\n";
const DATA: &str = "// plan: ";
const END: &str = "// gore npc routine end\n";

#[derive(Subcommand)]
pub enum RoutineAction {
    /// Insert or replace a daily phase in a workspace from npc new/clone
    Set {
        dir: PathBuf,
        /// Exact 24-hour time, HH:MM. The phase lasts until the next entry
        #[arg(long)]
        time: String,
        #[arg(long, value_enum)]
        activity: Activity,
        /// A named location; use routine spots to find compatible targets
        #[arg(long)]
        spot: String,
        /// Game install for checking object actions (sit/sleep/guard/alchemy)
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Show the daily phases and the explicit helper for an already-spawned NPC
    Show {
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Remove a phase; its predecessor then lasts until the next remaining phase
    Remove {
        dir: PathBuf,
        #[arg(long)]
        time: String,
        /// Game install for checking remaining object actions
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Find named locations compatible with an activity
    Spots {
        #[arg(long, value_enum)]
        activity: Activity,
        #[arg(long)]
        area: Option<String>,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long, default_value_t = 50)]
        max: usize,
        #[arg(long)]
        game: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

pub fn run(action: RoutineAction) -> Result<()> {
    match action {
        RoutineAction::Set {
            dir,
            time,
            activity,
            spot,
            game,
        } => {
            routine_plan::parse_time(&time)?;
            let _lock = lock_workspace(&dir)?;
            let mut workspace = RoutineWorkspace::load(&dir)?;
            let mut plan = workspace.current.plan.clone();
            plan.set(Phase {
                time,
                activity,
                spot,
            })?;
            canonicalize(&mut plan, game)?;
            workspace.save(plan)?;
            show(&dir, false)
        }
        RoutineAction::Remove { dir, time, game } => {
            let _lock = lock_workspace(&dir)?;
            let mut workspace = RoutineWorkspace::load(&dir)?;
            let mut plan = workspace.current.plan.clone();
            plan.remove(&time)?;
            canonicalize(&mut plan, game)?;
            workspace.save(plan)?;
            show(&dir, false)
        }
        RoutineAction::Show { dir, json } => show(&dir, json),
        RoutineAction::Spots {
            activity,
            area,
            prefix,
            max,
            game,
            json,
        } => {
            let catalog = catalog_for([activity], game)?;
            let list = catalog.list(activity, area.as_deref(), prefix.as_deref(), max)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                for spot in &list.spots {
                    println!(
                        "{}  {}  ({:.0}, {:.0}, {:.0})",
                        spot.name, spot.area, spot.x, spot.y, spot.z
                    );
                }
                println!(
                    "{} of {} shown; evidence: {}",
                    list.listed_count, list.matched_count, list.evidence
                );
                for note in &list.notes {
                    println!("{note}");
                }
            }
            Ok(())
        }
    }
}

fn catalog_for(
    activities: impl IntoIterator<Item = Activity>,
    game: Option<PathBuf>,
) -> Result<SpotCatalog> {
    let game = if activities.into_iter().any(needs_interaction_evidence) {
        Some(gore_loc::config::game_root(game).context("object routines require the installed InteractionSpots.json; configure game-path or pass --game")?)
    } else {
        None
    };
    SpotCatalog::load(game.as_deref())
}

fn canonicalize(plan: &mut Plan, game: Option<PathBuf>) -> Result<()> {
    plan.validate()?;
    let catalog = catalog_for(plan.phases.iter().map(|p| p.activity), game)?;
    for phase in &mut plan.phases {
        phase.spot = catalog
            .validate(phase.activity, &phase.spot)
            .with_context(|| format!("routine phase {} ({})", phase.time, phase.activity.as_str()))?
            .name;
    }
    Ok(())
}

fn show(dir: &Path, json: bool) -> Result<()> {
    let workspace = RoutineWorkspace::load(dir)?;
    let plan = &workspace.current.plan;
    let activation = routine_plan::activation_function(&workspace.manifest.npc_id);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "npc_id": workspace.manifest.npc_id, "phases": plan.phases,
                "managed": workspace.current.managed,
                "activation_function": if workspace.current.managed { Some(&activation) } else { None },
            }))?
        );
    } else if plan.phases.is_empty() {
        println!("No routine yet. Add its first phase with `gore npc routine set`. ");
    } else {
        println!("Daily routine for {}:", workspace.manifest.npc_id);
        for (index, phase) in plan.phases.iter().enumerate() {
            let next = &plan.phases[(index + 1) % plan.phases.len()].time;
            let wrap = if index + 1 == plan.phases.len() {
                " (next day)"
            } else {
                ""
            };
            println!(
                "  {} -> {}{}  {:7} {}",
                phase.time,
                next,
                wrap,
                phase.activity.as_str(),
                phase.spot
            );
        }
        if workspace.current.managed {
            println!("Walking, no randomized time offsets. Existing saves: call {activation}() explicitly from your dialog/script after spawning this NPC.");
            println!("Source prepared only; continue with npc check and npc stage, then compile/build/deploy.");
        } else {
            println!(
                "Legacy all-day routine; the first set/remove migrates it to a walking schedule."
            );
        }
    }
    Ok(())
}

fn block(plan: &Plan, id: &str) -> Result<String> {
    Ok(format!(
        "{BEGIN}{DATA}{}\n{}{END}",
        serde_json::to_string(plan)?,
        plan.render(id)?
    ))
}

struct CurrentRoutine {
    plan: Plan,
    range: Option<Range<usize>>,
    managed: bool,
}

/// Only our exact generated block (or the old one-phase generator) is editable. Handwritten
/// code is never inferred as a plan and silently regenerated away.
fn current_routine(source: &str, id: &str) -> Result<CurrentRoutine> {
    // The caller normalizes CRLF before calculating ranges and writing back in its original style.
    if source.contains("// gore npc routine begin") || source.contains("// gore npc routine end") {
        ensure!(source.matches(BEGIN).count() == 1 && source.matches(END).count() == 1,
            "routine markers are missing, duplicated or unsupported; restore the generated block before editing");
        let start = source.find(BEGIN).unwrap();
        let end = source.find(END).unwrap() + END.len();
        ensure!(
            start < end && (start == 0 || source.as_bytes()[start - 1] == b'\n'),
            "invalid routine block boundaries"
        );
        let contents = &source[start..end];
        let json = contents
            .strip_prefix(BEGIN)
            .unwrap()
            .lines()
            .next()
            .context("routine plan missing")?
            .strip_prefix(DATA)
            .context("routine plan metadata missing")?;
        let plan: Plan = serde_json::from_str(json).context("invalid routine plan metadata")?;
        ensure!(block(&plan, id)? == contents,
            "the generated routine block was manually changed; restore it before using routine commands (changes will not be overwritten)");
        let outside = format!("{}{}", &source[..start], &source[end..]);
        ensure!(
            !outside.contains(&format!("class UDailyRoutine_{id}_Start")),
            "duplicate routine class outside generated block"
        );
        return Ok(CurrentRoutine {
            plan,
            range: Some(start..end),
            managed: true,
        });
    }
    let header = format!("class UDailyRoutine_{id}_Start : UAIState_DailyRoutine_Human\n{{\n");
    let needle = format!("class UDailyRoutine_{id}_Start");
    if !source.contains(&needle) {
        return Ok(CurrentRoutine {
            plan: Plan::default(),
            range: None,
            managed: false,
        });
    }
    ensure!(
        source.matches(&needle).count() == 1,
        "duplicate routine class"
    );
    let start = source.find(&header).context("handwritten routine cannot be converted automatically; keep editing its AngelScript directly")?;
    let tail = &source[start + header.len()..];
    let prefix = "    default Schedule(0, 0, UAIState_Stand(), n\"";
    let spot = tail
        .strip_prefix(prefix)
        .and_then(|tail| tail.split_once('"'))
        .map(|(spot, _)| spot)
        .context(
            "only the original one-phase npc new/clone routine can be migrated automatically",
        )?;
    let old = format!("{header}{prefix}{spot}\", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);\n    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::WhenOutOfBounds;\n}}\n");
    ensure!(
        source[start..].starts_with(&old),
        "the legacy routine has manual changes; keep editing its AngelScript directly"
    );
    let plan = Plan {
        phases: vec![Phase {
            time: "00:00".into(),
            activity: Activity::Stand,
            spot: spot.into(),
        }],
    };
    plan.validate()?;
    Ok(CurrentRoutine {
        plan,
        range: Some(start..start + old.len()),
        managed: false,
    })
}

fn workspace_file(dir: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty()
            && Path::new(relative)
                .components()
                .all(|p| matches!(p, Component::Normal(_))),
        "workspace source path must be relative and stay inside the workspace: {relative:?}"
    );
    let root = dir.canonicalize()?;
    let path = root
        .join(relative)
        .canonicalize()
        .with_context(|| format!("reading workspace file {relative}"))?;
    ensure!(
        path.starts_with(&root),
        "workspace source escapes its directory: {relative}"
    );
    Ok(path)
}

/// Serialize routine commands in one workspace; editors are covered by the content checks below.
fn lock_workspace(dir: &Path) -> Result<fs::File> {
    let path = dir.canonicalize()?.join(".gore-npc-routine.lock");
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)?;
    file.try_lock()
        .with_context(|| format!("another routine command is editing {}", dir.display()))?;
    Ok(file)
}

struct RoutineWorkspace {
    manifest: Manifest,
    source_path: PathBuf,
    source_original: String,
    level_path: PathBuf,
    level_original: String,
    current: CurrentRoutine,
}

impl RoutineWorkspace {
    fn load(dir: &Path) -> Result<Self> {
        let manifest = read_manifest(dir)?;
        ensure!(matches!(manifest.operation, Operation::New | Operation::Clone),
            "routine commands edit workspaces from npc new/clone; shipped NPC checkout routines span other modules and need manual AngelScript edits");
        let authored = manifest
            .authored_module()
            .context("no authored NPC module in workspace")?;
        let level = manifest
            .level_edit()
            .context("no edited level in workspace")?;
        let source_path = workspace_file(dir, &authored.source_file)?;
        let level_path = workspace_file(dir, &level.source_file)?;
        ensure!(
            source_path != level_path,
            "NPC and level source must be separate files"
        );
        let source_original = fs::read_to_string(&source_path)?;
        let source = source_original.replace("\r\n", "\n");
        let level_original = fs::read_to_string(&level_path)?;
        let current = current_routine(&source, &manifest.npc_id)?;
        routine_plan::validate_identifier(&manifest.npc_id, "NPC id")?;
        wire_spawn(&level_original, &manifest, current.range.is_some())?;
        Ok(Self {
            manifest,
            source_path,
            source_original,
            level_path,
            level_original,
            current,
        })
    }

    fn save(&mut self, plan: Plan) -> Result<()> {
        let mut generated = block(&plan, &self.manifest.npc_id)?;
        let newline = if self.source_original.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        if newline == "\r\n" {
            generated = generated.replace('\n', newline);
        }
        let mut source = self.source_original.clone();
        if let Some(range) = self.current.range.clone() {
            let original =
                original_offset(&source, range.start)..original_offset(&source, range.end);
            source.replace_range(original, &generated);
        } else {
            source.push_str(newline);
            source.push_str(&generated);
        }
        let level = wire_spawn(
            &self.level_original,
            &self.manifest,
            self.current.range.is_some(),
        )?;
        // Claim both original files before checking either one. On Windows, exclusive sharing
        // prevents editors from writing or replacing the claimed paths until both writes finish.
        let mut source_file = ClaimedFile::open(&self.source_path, &self.source_original)?;
        let mut level_file = ClaimedFile::open(&self.level_path, &self.level_original)?;
        if source == self.source_original && level == self.level_original {
            return Ok(());
        }
        let mut source_written = false;
        let mut level_written = false;
        let result = (|| {
            // Unix advisory locks do not stop atomic-save renames or in-place edits. Check
            // both paths and their contents before and after each write.
            source_file.ensure_current_contents(&self.source_original)?;
            level_file.ensure_current_contents(&self.level_original)?;
            if source != self.source_original {
                write_or_restore(&mut source_file, &source, &self.source_original)
                    .context("writing NPC source")?;
                source_written = true;
            }
            source_file.ensure_current_contents(&source)?;
            level_file.ensure_current_contents(&self.level_original)?;
            if level != self.level_original {
                write_or_restore(&mut level_file, &level, &self.level_original)
                    .context("writing level source")?;
                level_written = true;
            }
            source_file.ensure_current_contents(&source)?;
            level_file.ensure_current_contents(&level)?;
            Ok(())
        })();
        if let Err(error) = result {
            if level_written {
                level_file
                    .restore_if_current(&level, &self.level_original)
                    .with_context(|| {
                        format!("routine update failed ({error}); restoring level failed")
                    })?;
            }
            if source_written {
                source_file
                    .restore_if_current(&source, &self.source_original)
                    .with_context(|| {
                        format!("routine update failed ({error}); restoring NPC failed")
                    })?;
            }
            return Err(error);
        }
        Ok(())
    }
}

/// Map normalized parser ranges back to the original bytes, preserving mixed line endings
/// in user-authored code outside the managed block.
fn original_offset(source: &str, normalized_offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut normalized = 0;
    for (offset, byte) in bytes.iter().enumerate() {
        if normalized == normalized_offset {
            return offset;
        }
        if *byte != b'\r' || bytes.get(offset + 1) != Some(&b'\n') {
            normalized += 1;
        }
    }
    source.len()
}

struct ClaimedFile {
    file: fs::File,
    path: PathBuf,
}

#[cfg(windows)]
fn link_count(file: &fs::File) -> Result<u64> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) };
    ensure!(
        ok != 0,
        "reading workspace file link count: {}",
        std::io::Error::last_os_error()
    );
    Ok(unsafe { info.assume_init() }.nNumberOfLinks as u64)
}

#[cfg(unix)]
fn link_count(file: &fs::File) -> Result<u64> {
    Ok(file.metadata()?.nlink())
}

impl ClaimedFile {
    fn open(path: &Path, expected: &str) -> Result<Self> {
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true);
        #[cfg(windows)]
        options.share_mode(0);
        let mut file = options
            .open(path)
            .with_context(|| format!("claiming {}", path.display()))?;
        file.try_lock()
            .with_context(|| format!("locking {}", path.display()))?;
        #[cfg(any(windows, unix))]
        ensure!(
            link_count(&file)? == 1,
            "workspace file has multiple hard links: {}",
            path.display()
        );
        let mut current = String::new();
        file.read_to_string(&mut current)?;
        ensure!(
            current == expected,
            "workspace file changed while editing: {}; retry the command",
            path.display()
        );
        let claimed = Self {
            file,
            path: path.to_path_buf(),
        };
        claimed.ensure_current_path()?;
        Ok(claimed)
    }

    fn write(&mut self, content: &str) -> Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(content.as_bytes())?;
        self.file.set_len(content.len() as u64)?;
        self.file.sync_all()?;
        Ok(())
    }

    fn read_contents(&mut self) -> Result<String> {
        let mut current = String::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_string(&mut current)?;
        Ok(current)
    }

    fn ensure_current_contents(&mut self, expected: &str) -> Result<()> {
        self.ensure_current_path()?;
        let current = self.read_contents()?;
        self.ensure_current_path()?;
        ensure!(
            current == expected,
            "workspace file changed while editing: {}; retry the command",
            self.path.display()
        );
        Ok(())
    }

    #[cfg(unix)]
    fn ensure_current_path(&self) -> Result<()> {
        let held = self.file.metadata()?;
        let current = fs::symlink_metadata(&self.path)
            .with_context(|| format!("workspace path disappeared: {}", self.path.display()))?;
        ensure!(
            held.dev() == current.dev() && held.ino() == current.ino(),
            "workspace path was replaced while editing: {}; retry the command",
            self.path.display()
        );
        Ok(())
    }

    #[cfg(not(unix))]
    fn ensure_current_path(&self) -> Result<()> {
        Ok(())
    }

    fn restore_if_current(&mut self, generated: &str, original: &str) -> Result<()> {
        // Never roll back a path now owned by an editor, or an in-place edit made later.
        if self.ensure_current_path().is_err() {
            return Ok(());
        }
        let current = self.read_contents()?;
        if current == generated {
            self.ensure_current_path()?;
            self.write(original)?;
            self.ensure_current_contents(original)?;
        }
        Ok(())
    }
}

fn write_or_restore(file: &mut ClaimedFile, content: &str, original: &str) -> Result<()> {
    file.ensure_current_contents(original)?;
    if let Err(error) = file.write(content) {
        file.restore_if_current(content, original)
            .with_context(|| format!("writing failed ({error}); safe restore also failed"))?;
        return Err(error);
    }
    file.ensure_current_contents(content)
}

#[cfg(test)]
mod replacement_tests {
    use super::*;

    #[test]
    fn claimed_write_checks_content_and_does_not_replace_later_edits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("npc.as");
        fs::write(&path, "original").unwrap();
        fs::write(&path, "editor change").unwrap();
        assert!(ClaimedFile::open(&path, "original").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "editor change");

        let mut claim = ClaimedFile::open(&path, "editor change").unwrap();
        #[cfg(windows)]
        assert!(fs::write(&path, "later edit").is_err());
        claim.write("generated").unwrap();
        drop(claim);
        assert_eq!(fs::read_to_string(&path).unwrap(), "generated");
    }

    #[test]
    fn claimed_write_rejects_hard_links() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("npc.as");
        let peer = dir.path().join("outside.as");
        fs::write(&path, "original").unwrap();
        fs::hard_link(&path, &peer).unwrap();
        assert!(ClaimedFile::open(&path, "original").is_err());
        assert_eq!(fs::read_to_string(&peer).unwrap(), "original");
    }

    #[cfg(unix)]
    #[test]
    fn claimed_write_detects_atomic_save_and_preserves_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("npc.as");
        let old = dir.path().join("old.as");
        fs::write(&path, "original").unwrap();
        let mut claim = ClaimedFile::open(&path, "original").unwrap();
        fs::rename(&path, &old).unwrap();
        fs::write(&path, "editor replacement").unwrap();
        assert!(claim.ensure_current_path().is_err());
        claim.restore_if_current("generated", "original").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "editor replacement");
    }

    #[cfg(unix)]
    #[test]
    fn claimed_write_detects_in_place_edit_and_preserves_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("npc.as");
        fs::write(&path, "original").unwrap();
        let mut claim = ClaimedFile::open(&path, "original").unwrap();
        fs::write(&path, "editor change").unwrap();
        assert!(write_or_restore(&mut claim, "generated", "original").is_err());
        claim.restore_if_current("generated", "original").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "editor change");
    }
}

fn wire_spawn(level: &str, manifest: &Manifest, had_routine: bool) -> Result<String> {
    let class = generate::spawn_class(&manifest.npc_id);
    let routine = format!("UDailyRoutine_{}_Start", manifest.npc_id);
    let expected = edit::spawn_line("", &class, if had_routine { Some(&routine) } else { None });
    let replacement = edit::spawn_line("", &class, Some(&routine));
    let mut matches = 0;
    let mut result = String::new();
    for line in level.split_inclusive('\n') {
        if line.contains("SpawnAIAgent(") && line.contains(&format!("({class}::StaticClass())")) {
            ensure!(
                line.trim() == expected.trim(),
                "spawn line was manually changed; cannot safely wire the routine: {}",
                line.trim()
            );
            matches += 1;
            let offset = line.find(line.trim_start()).unwrap();
            result.push_str(&line[..offset]);
            result.push_str(replacement.trim());
            if line.ends_with("\r\n") {
                result.push_str("\r\n");
            } else if line.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
        }
    }
    ensure!(
        matches == 1,
        "expected one generated spawn line for {}, found {matches}",
        manifest.npc_id
    );
    Ok(result)
}

/// Managed blocks are checked before staging as well as in npc check. Legacy and manually
/// authored routines remain under the existing checker and compiler contracts.
pub fn check_managed(dir: &Path, manifest: &Manifest, game: Option<PathBuf>) -> Result<()> {
    let Some(authored) = manifest.authored_module() else {
        return Ok(());
    };
    let source = fs::read_to_string(workspace_file(dir, &authored.source_file)?)?;
    if !source.contains("// gore npc routine") {
        return Ok(());
    }
    let workspace = RoutineWorkspace::load(dir)?;
    let mut plan = workspace.current.plan;
    canonicalize(&mut plan, game)?;
    Ok(())
}
