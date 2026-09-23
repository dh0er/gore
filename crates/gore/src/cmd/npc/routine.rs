//! Workspace editing for the bounded, generated daily-routine block.

use super::{
    defaults, edit, generate, read_manifest,
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

/// Preserve byte offsets while hiding comments and literals from declaration checks. Also
/// record block comments: the managed `//` marker lines themselves are valid line comments.
fn masked_script(source: &str) -> (String, Vec<Range<usize>>) {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        Line,
        Block(usize),
        Quote(u8),
    }

    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut blocks = Vec::new();
    let mut state = State::Code;
    let mut i = 0;
    while i < bytes.len() {
        let next = bytes.get(i + 1).copied();
        match state {
            State::Code if bytes[i] == b'/' && next == Some(b'/') => {
                masked[i..i + 2].fill(b' ');
                state = State::Line;
                i += 2;
            }
            State::Code if bytes[i] == b'/' && next == Some(b'*') => {
                masked[i..i + 2].fill(b' ');
                state = State::Block(i);
                i += 2;
            }
            State::Code if bytes[i] == b'"' || bytes[i] == b'\'' => {
                masked[i] = b' ';
                state = State::Quote(bytes[i]);
                i += 1;
            }
            State::Line if bytes[i] == b'\n' => {
                state = State::Code;
                i += 1;
            }
            State::Block(start) if bytes[i] == b'*' && next == Some(b'/') => {
                masked[i..i + 2].fill(b' ');
                blocks.push(start..i + 2);
                state = State::Code;
                i += 2;
            }
            State::Quote(_) if bytes[i] == b'\\' && next.is_some() => {
                masked[i..i + 2].fill(b' ');
                i += 2;
            }
            State::Quote(quote) if bytes[i] == quote => {
                masked[i] = b' ';
                state = State::Code;
                i += 1;
            }
            State::Code => i += 1,
            _ => {
                if bytes[i] != b'\n' && bytes[i] != b'\r' {
                    masked[i] = b' ';
                }
                i += 1;
            }
        }
    }
    if let State::Block(start) = state {
        blocks.push(start..bytes.len());
    }
    (String::from_utf8(masked).expect("masking preserves UTF-8"), blocks)
}

fn declares_function(code: &str, name: &str) -> bool {
    let code = code.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    code.match_indices(&name).any(|(at, _)| {
        let before = code[..at].trim_end();
        let return_type_start = before
            .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .map_or(0, |index| index + 1);
        if return_type_start == before.len()
            || !code[at + name.len()..].trim_start().starts_with('(') {
            return false;
        }
        // A method inside a class does not share the generated top-level function's scope.
        let depth = code[..at].bytes().fold(0i32, |depth, byte| match byte {
            b'{' => depth + 1,
            b'}' => depth - 1,
            _ => depth,
        });
        depth == 0
    })
}

fn ensure_no_generated_collisions(outside: &str, generated: &str, id: &str) -> Result<()> {
    let (code, _) = masked_script(outside);
    let existing: std::collections::HashSet<String> = defaults::parse_classes(&code)
        .into_iter()
        .map(|class| class.name.to_ascii_lowercase())
        .collect();
    for class in defaults::parse_classes(generated) {
        ensure!(
            !existing.contains(&class.name.to_ascii_lowercase()),
            "duplicate routine class outside generated block: {}",
            class.name
        );
    }
    let helper = routine_plan::activation_function(id);
    ensure!(
        !declares_function(&code, &helper),
        "duplicate routine function outside generated block: {helper}"
    );
    Ok(())
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
        let (_, block_comments) = masked_script(source);
        ensure!(
            !block_comments
                .iter()
                .any(|range| range.contains(&start) || range.contains(&(end - END.len()))),
            "routine markers are inside a block comment; restore the generated block before editing"
        );
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
        ensure_no_generated_collisions(&outside, contents, id)?;
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

#[cfg(test)]
mod routine_parse_tests {
    use super::*;
    use super::super::workspace::ModuleEdit;

    fn phase(activity: Activity) -> Phase {
        Phase {
            time: "08:00".into(),
            activity,
            spot: "WP_TEST".into(),
        }
    }

    #[test]
    fn markers_inside_block_comments_are_not_managed() {
        let generated = block(&Plan { phases: vec![phase(Activity::Stand)] }, "TEST").unwrap();
        let commented = format!("/*\n{generated}*/\n");
        assert!(current_routine(&commented, "TEST")
            .err().unwrap()
            .to_string()
            .contains("inside a block comment"));
        assert!(current_routine(&format!("/* unrelated */\n{generated}"), "TEST")
            .unwrap()
            .managed);
    }

    #[test]
    fn generated_classes_and_activation_function_cannot_be_redeclared() {
        let plan = Plan {
            phases: vec![
                Phase { time: "00:00".into(), ..phase(Activity::Stand) },
                Phase { time: "08:00".into(), ..phase(Activity::Read) },
                Phase { time: "12:00".into(), ..phase(Activity::Drink) },
            ],
        };
        let generated = block(&plan, "TEST").unwrap();
        for name in [
            "UAIState_GoreRoutine_TEST_Stand",
            "UAIState_GoreRoutine_TEST_Activity",
            "UAIState_GoreRoutine_TEST_Read",
            "UAIState_GoreRoutine_TEST_Drink",
            "UDailyRoutine_TEST_Start",
        ] {
            let outside = format!("class {name} : UObject\n{{}}\n");
            assert!(current_routine(&format!("{outside}{generated}"), "TEST").is_err(),
                "accepted duplicate {name}");
        }
        let duplicate = format!("bool GoreApplyRoutine_TEST() {{ return true; }}\n{generated}");
        assert!(current_routine(&duplicate, "TEST")
            .err().unwrap()
            .to_string()
            .contains("duplicate routine function"));
        assert!(current_routine(
            &format!("void GoreApplyRoutine_TEST() {{}}\n{generated}"),
            "TEST"
        ).is_err());
        let reference = format!("void CallRoutine() {{ GoreApplyRoutine_TEST(); }}\n{generated}");
        assert!(current_routine(&reference, "TEST").is_ok());
        let comments = format!(
            "/* class UAIState_GoreRoutine_TEST_Read : UObject {{}} */\n\
             // bool GoreApplyRoutine_TEST() {{}}\n{generated}"
        );
        assert!(current_routine(&comments, "TEST").is_ok());
    }

    #[test]
    fn newly_added_phase_checks_new_helper_names() {
        let original = block(&Plan { phases: vec![phase(Activity::Stand)] }, "TEST").unwrap();
        let outside = "class UAIState_GoreRoutine_TEST_Read : UObject\n{}\n";
        assert!(current_routine(&format!("{outside}{original}"), "TEST").is_ok());
        let next = block(&Plan { phases: vec![phase(Activity::Read)] }, "TEST").unwrap();
        assert!(ensure_no_generated_collisions(outside, &next, "TEST").is_err());
    }

    #[test]
    fn wired_spawn_requires_routine_class_when_managed_block_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = Manifest {
            operation: Operation::New,
            npc_id: "TEST".into(),
            derived_from: None,
            modules: vec![
                ModuleEdit {
                    module: "TEST".into(), relative_path: "TEST.as".into(),
                    source_file: "TEST.as".into(), pristine_file: None, op: "add".into(),
                },
                ModuleEdit {
                    module: "Level".into(), relative_path: "Level.as".into(),
                    source_file: "Level.as".into(), pristine_file: None, op: "edit".into(),
                },
            ],
            world_points: vec![],
            level_module: "Level".into(),
            cache_sha256: "0".repeat(64),
            modular_visuals: false,
        };
        let source_path = dir.path().join("TEST.as");
        let level_path = dir.path().join("Level.as");
        fs::write(&source_path, "class UCharacterDefinition_TEST : UObject\n{}\n").unwrap();
        fs::write(&level_path, edit::spawn_line("", &generate::spawn_class("TEST"), Some("UDailyRoutine_TEST_Start"))).unwrap();
        assert!(check_managed(dir.path(), &manifest, None)
            .unwrap_err().to_string().contains("routine class is missing"));

        fs::write(&source_path, "// class UDailyRoutine_TEST_Start : UObject\n").unwrap();
        assert!(check_managed(dir.path(), &manifest, None).is_err());
        fs::write(&source_path, "class UDailyRoutine_TEST_Start : UAIState_DailyRoutine_Human\n{}\n").unwrap();
        check_managed(dir.path(), &manifest, None).unwrap();

        fs::write(&source_path, "class UCharacterDefinition_TEST : UObject\n{}\n").unwrap();
        fs::write(&level_path, edit::spawn_line("", &generate::spawn_class("TEST"), None)).unwrap();
        check_managed(dir.path(), &manifest, None).unwrap();
    }
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
        let normalized = self.source_original.replace("\r\n", "\n");
        let outside = if let Some(range) = &self.current.range {
            format!("{}{}", &normalized[..range.start], &normalized[range.end..])
        } else {
            normalized
        };
        ensure_no_generated_collisions(&outside, &generated, &self.manifest.npc_id)?;
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
                    .restore_if_current(level.as_bytes(), &self.level_original)
                    .with_context(|| {
                        format!("routine update failed ({error}); restoring level failed")
                    })?;
            }
            if source_written {
                source_file
                    .restore_if_current(source.as_bytes(), &self.source_original)
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

    fn read_contents(&mut self) -> Result<Vec<u8>> {
        let mut current = Vec::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_end(&mut current)?;
        Ok(current)
    }

    fn ensure_current_contents(&mut self, expected: &str) -> Result<()> {
        self.ensure_current_path()?;
        let current = self.read_contents()?;
        self.ensure_current_path()?;
        ensure!(
            current == expected.as_bytes(),
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

    fn restore_if_current(&mut self, generated: &[u8], original: &str) -> Result<()> {
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
    file.file.seek(SeekFrom::Start(0))?;
    let bytes = content.as_bytes();
    let mut written = 0;
    let mut resized = false;
    let result = (|| -> Result<()> {
        while written < bytes.len() {
            match file.file.write(&bytes[written..]) {
                Ok(0) => return Err(std::io::Error::from(std::io::ErrorKind::WriteZero).into()),
                Ok(count) => written += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        file.file.set_len(bytes.len() as u64)?;
        resized = true;
        file.file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        // A failed write_all or set_len can leave only a prefix of our new bytes. Restore
        // the original only when the held file still has exactly those bytes, including a
        // possible old tail. Otherwise an editor has changed it and owns the current data.
        let partial = bytes_after_partial_write(original.as_bytes(), bytes, written, resized);
        file.restore_if_current(&partial, original)
            .with_context(|| format!("writing failed ({error}); safe restore also failed"))?;
        return Err(error);
    }
    file.ensure_current_contents(content)
}

fn bytes_after_partial_write(
    original: &[u8],
    generated: &[u8],
    written: usize,
    resized: bool,
) -> Vec<u8> {
    if resized {
        return generated.to_vec();
    }
    let mut partial = original.to_vec();
    partial.resize(original.len().max(written), 0);
    partial[..written].copy_from_slice(&generated[..written]);
    partial
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
        claim.restore_if_current(b"generated", "original").unwrap();
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
        claim.restore_if_current(b"generated", "original").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "editor change");
    }

    #[test]
    fn partial_write_or_failed_truncation_restores_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("npc.as");
        let original = "original source";
        let generated = "☀ new source";
        fs::write(&path, original).unwrap();
        let mut claim = ClaimedFile::open(&path, original).unwrap();
        claim.file.seek(SeekFrom::Start(0)).unwrap();
        claim.file.write_all(&generated.as_bytes()[..1]).unwrap();
        let partial =
            bytes_after_partial_write(original.as_bytes(), generated.as_bytes(), 1, false);
        assert_eq!(claim.read_contents().unwrap(), partial);
        claim.restore_if_current(&partial, original).unwrap();
        drop(claim);
        assert_eq!(fs::read_to_string(&path).unwrap(), original);

        let mut claim = ClaimedFile::open(&path, original).unwrap();
        let short = "new";
        claim.file.seek(SeekFrom::Start(0)).unwrap();
        claim.file.write_all(short.as_bytes()).unwrap();
        let untrimmed =
            bytes_after_partial_write(original.as_bytes(), short.as_bytes(), short.len(), false);
        assert_eq!(claim.read_contents().unwrap(), untrimmed);
        claim.restore_if_current(&untrimmed, original).unwrap();
        drop(claim);
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
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
        if let Some(level) = manifest.level_edit() {
            let level_source = fs::read_to_string(workspace_file(dir, &level.source_file)?)?;
            let routine = format!("UDailyRoutine_{}_Start", manifest.npc_id);
            let spawn = edit::spawn_line("", &generate::spawn_class(&manifest.npc_id), Some(&routine));
            if level_source.lines().any(|line| line.trim() == spawn) {
                ensure!(
                    defaults::parse_classes(&source)
                        .iter()
                        .any(|class| class.namespace.is_none() && class.name == routine),
                    "spawn references {routine}, but its routine class is missing from the authored source"
                );
            }
        }
        return Ok(());
    }
    let workspace = RoutineWorkspace::load(dir)?;
    let mut plan = workspace.current.plan;
    canonicalize(&mut plan, game)?;
    Ok(())
}
