//! The documentation, compiled into the binary.
//!
//! Embedded rather than read from disk because the release zip unpacks wherever the user likes and
//! the server has no reliable way to find `docs/` afterwards. `include_str!` also registers each
//! file for rebuild tracking, so editing a page rebuilds the crate.
//!
//! Both bodies are here. `docs/guide/` is the user guide — instructions, and the only thing that
//! ships in the zip or is rendered by `gore guide html`. `docs/reference/` is the contract behind
//! those instructions, and an agent needs it for the case the guide deliberately does not cover:
//! a command refused something and the reason is an invariant, not a typo.
//!
//! The list is written out by hand instead of globbed by a build script: a build script would buy
//! only auto-discovery, and the test below already provides auto-discovery's real benefit — it
//! reads both directories at test time and fails if this list and the tree disagree.

use super::{Kind, Page};

macro_rules! pages {
    ($($kind:ident / $slug:literal => $path:literal),* $(,)?) => {
        /// Every embedded page, guide first, each body in reading order.
        pub const PAGES: &[Page] = &[$(
            Page {
                slug: $slug,
                file: $path,
                kind: Kind::$kind,
                markdown: include_str!(concat!("../../../../docs/", $path)),
            },
        )*];
    };
}

pages! {
    // ---- Guide: how to mod the game -------------------------------------------------------
    // Basics
    Guide / "README"               => "guide/README.md",
    Guide / "getting-started"      => "guide/getting-started.md",
    Guide / "cli-reference"        => "guide/cli-reference.md",
    Guide / "find"                 => "guide/find.md",
    Guide / "dialog-trees"         => "guide/dialog-trees.md",
    Guide / "mcp"                  => "guide/mcp.md",
    // Modding domains
    Guide / "items"                => "guide/items.md",
    Guide / "npc-authoring"        => "guide/npc-authoring.md",
    Guide / "text-and-dialogs"     => "guide/text-and-dialogs.md",
    Guide / "audio"                => "guide/audio.md",
    Guide / "voice"                => "guide/voice.md",
    Guide / "textures"             => "guide/textures.md",
    Guide / "dataassets"           => "guide/dataassets.md",
    Guide / "scripts"              => "guide/scripts.md",
    // AngelScript authoring
    Guide / "dialog-authoring"     => "guide/dialog-authoring.md",
    Guide / "angelscript-defaults" => "guide/angelscript-defaults.md",
    // Shipping and combining
    Guide / "bundles"              => "guide/bundles.md",
    Guide / "mod-manager"          => "guide/mod-manager.md",
    // Also
    Guide / "mod-studio"           => "guide/mod-studio.md",
    Guide / "catalogs-and-models"  => "guide/catalogs-and-models.md",

    // ---- Reference: the contracts behind those commands -----------------------------------
    Reference / "dataassets-internals"    => "reference/dataassets-internals.md",
    Reference / "angelscript-internals"   => "reference/angelscript-internals.md",
    Reference / "game-updates"            => "reference/game-updates.md",
    Reference / "dialog-runtime"          => "reference/dialog-runtime.md",
    Reference / "studio-authoring"        => "reference/studio-authoring.md",
    Reference / "studio-voice"            => "reference/studio-voice.md",
    Reference / "studio-project-archive"  => "reference/studio-project-archive.md",
    Reference / "survival-mode"           => "reference/survival-mode.md",
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const GORE_MODDING_SKILL: &str =
        include_str!("../../../../plugins/gore/skills/gore-modding/SKILL.md");

    /// `docs/reference/README.md` is an index for browsing the directory on GitHub, not content.
    /// Embedding it would also collide with the guide's own README slug.
    const NOT_EMBEDDED: &[&str] = &["reference/README.md"];

    /// One documentation directory as it exists on disk, read at test time.
    fn on_disk(subdir: &str) -> BTreeSet<String> {
        let dir = format!("{}/../../docs/{subdir}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("cannot read {dir}: {error}"))
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with(".md"))
            .map(|name| format!("{subdir}/{name}"))
            .filter(|path| !NOT_EMBEDDED.contains(&path.as_str()))
            .collect()
    }

    #[test]
    fn the_embedded_pages_are_exactly_the_pages_on_disk() {
        // This is what makes the hand-written list safe: add a page to docs/guide or docs/reference
        // and forget it here, and the next `cargo test` says so.
        let embedded: BTreeSet<String> = PAGES.iter().map(|page| page.file.to_string()).collect();
        let mut disk = on_disk("guide");
        disk.extend(on_disk("reference"));
        assert_eq!(
            embedded, disk,
            "docs/ and the embedded page list have diverged"
        );
    }

    #[test]
    fn slugs_are_unique_and_derived_from_the_filename() {
        // Unique across both bodies, because a slug alone must name exactly one page: it is what
        // `gore_guide` takes as `page`, and search hits carry nothing else.
        let mut seen = BTreeSet::new();
        for page in PAGES {
            assert!(seen.insert(page.slug), "duplicate slug {}", page.slug);
            let stem = page.file.rsplit('/').next().expect("a filename");
            assert_eq!(
                stem,
                format!("{}.md", page.slug),
                "the slug must be the filename without its extension"
            );
        }
    }

    #[test]
    fn every_page_sits_under_the_directory_its_kind_names() {
        for page in PAGES {
            let expected = match page.kind {
                Kind::Guide => "guide/",
                Kind::Reference => "reference/",
            };
            assert!(
                page.file.starts_with(expected),
                "{} is {:?} but lives in {}",
                page.slug,
                page.kind,
                page.file
            );
        }
    }

    #[test]
    fn both_bodies_are_populated() {
        assert!(super::super::pages_of(Kind::Guide).count() >= 10);
        assert!(super::super::pages_of(Kind::Reference).count() >= 1);
    }

    #[test]
    fn no_page_is_empty() {
        for page in PAGES {
            assert!(!page.markdown.trim().is_empty(), "{} is empty", page.slug);
        }
    }

    #[test]
    fn scripts_guide_separates_standalone_diagnostics_from_the_game_hook() {
        let scripts = PAGES
            .iter()
            .find(|page| page.slug == "scripts")
            .expect("scripts guide page")
            .markdown;

        for claim in [
            "source file, line, column, severity, and message",
            "diagnostics require no game launch and no consent question",
            "hook belongs only to the game backend",
            "strict standalone compilation never loads it",
        ] {
            assert!(scripts.contains(claim), "scripts guide lost: {claim}");
        }
    }

    #[test]
    fn doctor_studio_and_plugin_keep_the_strict_standalone_contract() {
        let page = |slug| {
            PAGES
                .iter()
                .find(|page| page.slug == slug)
                .unwrap_or_else(|| panic!("missing {slug} guide page"))
                .markdown
        };

        for claim in [
            "based on the cache/API, not a whole-file Steam/GOG executable checksum",
            "native file/line/column/severity",
            "Doctor does not run a",
        ] {
            assert!(
                page("cli-reference").contains(claim),
                "Doctor guide lost: {claim}"
            );
        }
        for claim in [
            "Strict standalone returns native file, line, column",
            "without starting the game or entering an",
            "Neither route",
            "grants build, deployment, or runtime authority",
        ] {
            assert!(
                page("mod-studio").contains(claim),
                "Studio guide lost: {claim}"
            );
        }
        for claim in [
            "gore_as_compile` or",
            "never need game-launch consent",
            "does not start the game",
            "returns native compiler diagnostics",
            "runtime diagnostics hook belongs only to the game backend",
        ] {
            assert!(
                GORE_MODDING_SKILL.contains(claim),
                "plugin skill lost: {claim}"
            );
        }
    }

    #[test]
    fn generation_references_keep_the_latest_offline_boundary_explicit() {
        let game_updates = PAGES
            .iter()
            .find(|page| page.slug == "game-updates")
            .expect("game-updates reference page")
            .markdown;
        let studio = PAGES
            .iter()
            .find(|page| page.slug == "studio-authoring")
            .expect("studio-authoring reference page")
            .markdown;

        for page in [game_updates, studio] {
            for claim in [
                "exactly four reviewed",
                "24878692",
                "fresh 172709 USMAP",
                "UGameplayAbilityAttackLoop",
                "UGameplayAbilityMagicBase",
                "UGameplayAbilityCastSpell",
                "URagdollConfig",
                "26,399 → 26,389",
            ] {
                assert!(page.contains(claim), "generation reference lost: {claim}");
            }
        }

        for claim in [
            "-14 + 4 = -10",
            "frozen 27-case and full-tree embedded-versus-standalone comparisons",
            "neither build `24340829` nor build `24878692`",
        ] {
            assert!(
                game_updates.contains(claim),
                "game update contract lost: {claim}"
            );
        }

        for claim in [
            "bounded project-only Story/NPC/Quest",
            "no dialog-runtime, production-build, deployment, live-game, or",
            "Standalone compiler-core parity is",
            "separate frozen-corpus/full-tree qualification",
        ] {
            assert!(studio.contains(claim), "Studio boundary lost: {claim}");
        }
    }
}
