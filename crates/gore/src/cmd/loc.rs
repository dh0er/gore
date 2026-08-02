//! `gore-cli loc` — read and edit the game's localized text directly from the
//! encrypted AlkimiaLocalization `.lcache` (no game run needed).
//!
//! - `export`: decrypt + flatten to `{ text_id: { language: value } }` for every
//!   id and language the game ships. Consumed by gore-save and gore-mod.
//! - `import`: apply `{ id: { language: value } }` edits and re-encrypt the
//!   `.lcache` (a static text / translation mod). Unedited fields keep their
//!   original bytes, so a no-edit round-trip is byte-identical.
//!
//! All three take `--lcache` and none of them require it: the cache is found
//! the same way every other command finds the install.

use anyhow::{bail, Context, Result};
use gore_loc::loc::Lcache;
use gore_loc::{config, loc_store, paths};
use std::io::Write as _;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

type LocMap = BTreeMap<String, BTreeMap<String, String>>;

struct PendingLoc {
    /// Most recently encountered spelling of this case-insensitive id.
    id: String,
    /// Folded language -> (most recent spelling, text).
    values: BTreeMap<String, (String, String)>,
}

/// JSON object keys are case-sensitive, while lcache ids and language names are not. Collapse all
/// aliases before touching the cache so a newly-added id receives the union of its translations in
/// one atomic `add_key` call. BTreeMap traversal keeps duplicate-alias resolution deterministic.
fn fold_loc_aliases(edits: LocMap) -> BTreeMap<String, PendingLoc> {
    let mut folded = BTreeMap::new();
    for (id, languages) in edits {
        let pending = folded
            .entry(id.to_ascii_lowercase())
            .or_insert_with(|| PendingLoc {
                id: id.clone(),
                values: BTreeMap::new(),
            });
        pending.id = id;
        for (language, text) in languages {
            pending
                .values
                .insert(language.to_ascii_lowercase(), (language, text));
        }
    }
    folded
}

/// Auto-detect (or use `--lcache`) the game's localization cache and write the
/// shared `gore/loc_catalog.json`. Prompts for confirmation unless `--yes`.
pub fn extract(lcache: Option<PathBuf>, yes: bool) -> Result<()> {
    let (resolved, _) = require_lcache(lcache.as_deref())?;

    if !yes {
        println!("Extract localized text from:\n  {}", resolved.display());
        println!(
            "into shared catalog:\n  {}",
            paths::loc_catalog_path().display()
        );
        print!("Proceed? [y/N] ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let meta = loc_store::extract(Some(&resolved)).context("extracting localization")?;
    println!(
        "Extracted {} ids across {} languages -> {}",
        meta.id_count,
        meta.languages.len(),
        meta.catalog_path
    );
    Ok(())
}

/// The `.lcache` a `loc` command will read, plus whether `--lcache` named that
/// exact file.
///
/// All three commands resolve it the same way, which is the point: `extract`
/// auto-detected from the first day and `export`/`import` did not, so a session
/// that reached for `export` spent its calls discovering that the file lives at
/// `G1R\Story\Cache\AlkimiaLocalization_00000000.lcache` — a path nothing in the
/// tool prints.
///
/// The flag matters to `import`, which rewrites its input when given no `--out`.
/// "In place" must mean a file the caller pointed at, never one this process
/// went looking for.
fn require_lcache(lcache: Option<&Path>) -> Result<(PathBuf, bool)> {
    // A `.lcache` file named outright is that file, whatever it is called. Auto-detection only
    // recognises the installed spelling (`AlkimiaLocalization*.lcache`) and walks a non-matching
    // file's ancestors looking for one — right for a hint that points at the game executable,
    // wrong for `export --lcache backup.lcache` or an `import` into a working copy.
    if let Some(hint) = lcache {
        let is_cache_file = hint
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lcache"));
        if is_cache_file && hint.is_file() {
            return Ok((hint.to_path_buf(), true));
        }
    }
    let Some(resolved) = resolve_lcache(lcache) else {
        // A hint that resolved to nothing is a different failure from finding nothing at all,
        // and the only one where the caller has a path to check.
        if let Some(hint) = lcache {
            bail!(
                "no AlkimiaLocalization .lcache at or under '{}' — pass the .lcache itself, the \
                 game dir, or a Steam library, or omit --lcache to auto-detect",
                hint.display()
            );
        }
        bail!(
            "no AlkimiaLocalization .lcache found (tried --lcache, the configured \
             game path, then Steam auto-detect). Pass --lcache <path-to-.lcache or game dir>."
        );
    };
    Ok((resolved, false))
}

/// Resolve the `.lcache`, mirroring every other command's game-path
/// precedence: explicit `--lcache` > the configured `game_path` > Steam
/// auto-detect. The configured path is normalized to the install root via
/// [`config::game_root`] (so an exe / `Win64` / intermediate path resolves the
/// same as it does for `mod`/`mgr`/`texture`), and each level falls back to the
/// next when it can't resolve a cache — so a stale configured path never blocks
/// extraction, it just yields to Steam auto-detect.
fn resolve_lcache(lcache: Option<&Path>) -> Option<PathBuf> {
    // 1. An explicit --lcache is authoritative: the user pointed us at it.
    if let Some(hint) = lcache {
        return loc_store::resolve_lcache(Some(hint));
    }
    // 2. The configured game path (else Steam), normalized to the G1R-containing
    //    root exactly like the other commands, then find the cache under it.
    if let Ok(root) = config::game_root(None) {
        if let Some(found) = loc_store::resolve_lcache(Some(&root)) {
            return Some(found);
        }
    }
    // 3. Fall back to a direct Steam `.lcache` scan (covers a stale configured
    //    path, or an install whose root normalization missed but discover finds)
    //    — but honor the autodetect-disable seam, so a caller that excluded Steam
    //    (tests / power users) never has `loc` reach an install behind their back.
    if config::autodetect_disabled() {
        return None;
    }
    loc_store::resolve_lcache(None)
}

/// What an in-place import owes its caller before it writes: the path, when nothing on the
/// command line named it and nothing redirects the result.
///
/// Without `--out` the input IS the output, and with `--lcache` optional the input can now be a
/// file the caller never saw chosen — the installation's only copy, which this command replaces
/// without a backup. `None` when the caller typed the path or passed `-o`, because then they
/// already know where this lands.
fn in_place_notice(lcache: &Path, named: bool, out: Option<&Path>) -> Option<String> {
    (out.is_none() && !named).then(|| {
        format!(
            "Rewriting the auto-detected .lcache in place, keeping no backup:\n  {}\n  \
             (pass -o <file> to write a new .lcache instead)",
            lcache.display()
        )
    })
}

/// Print whether a shared catalog exists and its provenance.
pub fn status() -> Result<()> {
    // Key off the catalog file (like the apps), so a leftover loc_meta.json
    // without its catalog isn't reported as an extracted catalog.
    if !loc_store::catalog_present() {
        println!(
            "no loc catalog extracted yet -> run `gore-cli loc extract` (shared dir: {})",
            paths::shared_data_dir().display()
        );
        return Ok(());
    }
    match loc_store::status() {
        Some(m) => {
            println!("loc catalog: present");
            println!("  ids:        {}", m.id_count);
            println!(
                "  languages:  {} [{}]",
                m.languages.len(),
                m.languages.join(", ")
            );
            println!("  source:     {} ({} bytes)", m.source_path, m.source_bytes);
            println!("  extracted:  {} (unix)", m.extracted_at);
            println!("  path:       {}", m.catalog_path);
        }
        None => {
            // Catalog exists but its metadata doesn't (e.g. a catalog write that
            // succeeded before the meta write failed).
            println!("loc catalog: present (no metadata)");
            println!("  path:       {}", paths::loc_catalog_path().display());
        }
    }
    Ok(())
}

pub fn export(lcache: Option<PathBuf>, out: PathBuf, keep_empty: bool) -> Result<()> {
    let (lcache, _) = require_lcache(lcache.as_deref())?;
    let enc =
        fs::read(&lcache).with_context(|| format!("reading lcache '{}'", lcache.display()))?;
    let lc = Lcache::decode(&enc).context("decoding lcache")?;
    let map = lc.export(keep_empty);
    // Pretty, not compact. The whole cache is ~44 000 ids in every shipped language, and written
    // as one line that is 27 MB of it: every grep for a line of dialog matches line 1 and comes
    // back as one unreadable blob, so the file can be searched but never read. One id per block
    // and one language per line costs about a fifth more bytes and makes the export usable by the
    // tools anyone actually has.
    fs::write(&out, serde_json::to_vec_pretty(&map).context("serializing")?)
        .with_context(|| format!("writing '{}'", out.display()))?;
    println!(
        "Exported {} ids across {} languages [{}]\n  from {}\n  ->   {}",
        map.len(),
        lc.languages().len(),
        lc.languages().join(", "),
        lcache.display(),
        out.display()
    );
    Ok(())
}

pub fn import(
    lcache: Option<PathBuf>,
    edits: PathBuf,
    out: Option<PathBuf>,
    add_missing: bool,
) -> Result<()> {
    let (lcache, named) = require_lcache(lcache.as_deref())?;
    if let Some(notice) = in_place_notice(&lcache, named, out.as_deref()) {
        println!("{notice}");
    }
    let enc =
        fs::read(&lcache).with_context(|| format!("reading lcache '{}'", lcache.display()))?;
    let mut lc = Lcache::decode(&enc).context("decoding lcache")?;

    let edits_json = fs::read_to_string(&edits)
        .with_context(|| format!("reading edits '{}'", edits.display()))?;
    let edits: LocMap = serde_json::from_str(&edits_json)
        .context("parsing edits (expected {\"id\":{\"lang\":\"text\"}})")?;
    let edits = fold_loc_aliases(edits);

    let mut applied = 0usize;
    for pending in edits.values() {
        let key = &pending.id;
        let langs: BTreeMap<String, String> = pending
            .values
            .values()
            .map(|(language, text)| (language.clone(), text.clone()))
            .collect();
        if add_missing && !langs.is_empty() && !lc.has_key(key) {
            // Add a new id atomically with all of its translations. This keeps
            // the file's header language order and prevents a bad language late
            // in the map from leaving a partially-built record in memory.
            lc.add_key(key, &langs)
                .with_context(|| format!("adding {key}"))?;
            applied += langs.len();
            continue;
        }
        for (lang, text) in &langs {
            lc.set_value(key, lang, text)
                .with_context(|| format!("editing {key}/{lang}"))?;
            applied += 1;
        }
    }

    let out_path = out.unwrap_or(lcache);
    // Write via temp + rename so an interrupted/failed write never truncates the
    // only game .lcache in place (import overwrites it directly without --out).
    let bytes = lc.encode().context("encoding lcache")?;
    loc_store::write_atomic(&out_path, &bytes)
        .with_context(|| format!("writing '{}'", out_path.display()))?;
    println!("Applied {applied} edit(s) -> {}", out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
    use aes::Aes256;

    const TEST_LCACHE_AES_KEY: &[u8; 32] = b"8f93ff6fa254d9c536ad88c1ff1d812b";

    fn fstring(text: &str) -> Vec<u8> {
        if text.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        let mut out = (bytes.len() as i32).to_le_bytes().to_vec();
        out.extend_from_slice(&bytes);
        out
    }

    fn empty_lcache() -> Vec<u8> {
        let mut plain = Vec::new();
        plain.push(0);
        plain.extend_from_slice(&(b"LCACHE".len() as i32).to_le_bytes());
        plain.extend_from_slice(b"LCACHE");
        plain.extend_from_slice(&2i32.to_le_bytes());
        plain.extend_from_slice(&fstring("german"));
        plain.extend_from_slice(&fstring("english"));
        plain.extend_from_slice(&0i32.to_le_bytes());
        let pad = (16 - plain.len() % 16) % 16;
        plain.extend(std::iter::repeat_n(0u8, pad));

        let cipher = Aes256::new(GenericArray::from_slice(TEST_LCACHE_AES_KEY));
        for block in plain.chunks_mut(16) {
            cipher.encrypt_block(GenericArray::from_mut_slice(block));
        }
        plain
    }

    /// An lcache holding one line, written where the test says and named what the test says.
    fn lcache_with_one_line(path: &Path, id: &str, german: &str) {
        let mut lc = Lcache::decode(&empty_lcache()).unwrap();
        let mut langs = BTreeMap::new();
        langs.insert("german".to_string(), german.to_string());
        lc.add_key(id, &langs).unwrap();
        fs::write(path, lc.encode().unwrap()).unwrap();
    }

    #[test]
    fn an_exported_line_of_dialog_can_be_read_by_the_tool_that_found_it() {
        // The session this exists for: a 27 MB export written as ONE line. All six greps for a
        // line of dialog matched line 1, five came back "[Omitted long matching line]", and the
        // voice swap was planned around a German line nobody ever read. Compact JSON makes the
        // file searchable and unreadable at the same time, which is the worst of the two.
        let dir = tempfile::tempdir().unwrap();
        // Deliberately not the installed spelling: a `.lcache` named outright is that file, so
        // this also pins that `--lcache` still means what it always meant.
        let input = dir.path().join("working-copy.lcache");
        let out = dir.path().join("loc.json");
        lcache_with_one_line(&input, "info_diego_gamestart_11_00", "Bleib stehen!");

        super::export(Some(input), out.clone(), false).unwrap();

        let text = fs::read_to_string(&out).unwrap();
        let hit = text
            .lines()
            .find(|line| line.contains("Bleib stehen!"))
            .unwrap_or_else(|| panic!("the exported text is on no line of its own:\n{text}"));
        assert!(
            hit.len() < 200,
            "a grep hit has to be short enough to print: {hit}"
        );
        // Still JSON, and still the same JSON.
        let parsed: LocMap = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["info_diego_gamestart_11_00"]["german"], "Bleib stehen!");
    }

    #[test]
    fn a_lcache_hint_that_resolves_to_nothing_names_the_path_that_was_tried() {
        // Routing `--lcache` through the resolver is what makes the flag optional, and it is also
        // how a typo'd path stopped being "the system cannot find the file" and became
        // "auto-detect found nothing" — a sentence about a search the caller did not ask for.
        let dir = tempfile::tempdir().unwrap();
        let nowhere = dir.path().join("Story/Cache");
        let error = require_lcache(Some(&nowhere)).unwrap_err().to_string();
        assert!(
            error.contains(&nowhere.display().to_string()),
            "the failure must name the path it was given: {error}"
        );
    }

    #[test]
    fn an_import_pointed_at_a_directory_finds_the_installed_cache_and_rewrites_it() {
        // The whole of finding 7 for `import`: `--lcache` is optional and also takes a game dir
        // or a Steam library, so nobody has to know that the file is called
        // `AlkimiaLocalization_00000000.lcache` and lives under `G1R\Story\Cache`.
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("Story/Cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let installed = cache_dir.join("AlkimiaLocalization_00000000.lcache");
        lcache_with_one_line(&installed, "goremod_id", "Alt");
        let edits = dir.path().join("edits.json");
        fs::write(&edits, br#"{"goremod_id":{"german":"Neu"}}"#).unwrap();

        super::import(Some(cache_dir), edits, None, false).unwrap();

        let after = Lcache::decode(&fs::read(&installed).unwrap()).unwrap();
        assert_eq!(after.export(false)["goremod_id"]["german"], "Neu");
    }

    #[test]
    fn an_import_says_which_cache_it_overwrites_when_it_chose_the_cache_itself() {
        // In place with no backup is the documented default, and it was reachable only by typing
        // the path. Now that the path can come from a search the caller never saw, the write is
        // the first moment they would learn which file it was — unless it is said first.
        let named = Path::new("D:/keep/AlkimiaLocalization_00000000.lcache");
        assert!(
            in_place_notice(named, true, None).is_none(),
            "a path the caller typed is not news"
        );
        assert!(
            in_place_notice(named, false, Some(Path::new("new.lcache"))).is_none(),
            "-o already says where the result lands"
        );
        let notice = in_place_notice(named, false, None)
            .expect("a found cache with no -o is exactly the case worth saying out loud");
        assert!(
            notice.contains("AlkimiaLocalization_00000000.lcache") && notice.contains("-o"),
            "the notice must name the file and the way out: {notice}"
        );
    }

    #[test]
    fn import_add_missing_folds_id_and_language_aliases_before_insert() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("base.lcache");
        let edits = dir.path().join("edits.json");
        let output = dir.path().join("edited.lcache");
        fs::write(&input, empty_lcache()).unwrap();
        fs::write(
            &edits,
            serde_json::to_vec(&serde_json::json!({
                "GOREMOD_CASE_ID": {"German": "Erste Zeile"},
                "goremod_case_id": {
                    "english": "English line",
                    "german": "Zweite Zeile"
                }
            }))
            .unwrap(),
        )
        .unwrap();

        super::import(Some(input), edits, Some(output.clone()), true).unwrap();

        let decoded = Lcache::decode(&fs::read(output).unwrap()).unwrap();
        let exported = decoded.export(false);
        let matches: Vec<_> = exported
            .iter()
            .filter(|(id, _)| id.eq_ignore_ascii_case("goremod_case_id"))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "case aliases must create one lcache group"
        );
        assert_eq!(matches[0].1["german"], "Zweite Zeile");
        assert_eq!(matches[0].1["english"], "English line");
    }
}
