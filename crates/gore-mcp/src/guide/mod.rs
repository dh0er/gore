//! The embedded guide: pages, their sections, and full-text search over them.
//!
//! The guide is what turns this server from a remote control into something an agent can use
//! correctly. The CLI's `--help` lists flags; the guide explains which command to reach for, in
//! what order, and what will break if you skip a step. Making it reachable is therefore not a
//! convenience feature — it is the difference between an agent that runs commands and one that
//! knows why.

pub mod pages;
pub mod search;

pub use pages::PAGES;

/// Which body of documentation a page belongs to.
///
/// Both are served here, because an agent needs both: the guide says which command to reach for,
/// and the reference says why the command refused. They stay separate everywhere a human looks —
/// only the guide ships in the release zip and only the guide is rendered by `gore guide html`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `docs/guide/` — instructions for someone modding the game.
    Guide,
    /// `docs/reference/` — implementation contracts and invariants behind those instructions.
    Reference,
}

impl Kind {
    /// The URI namespace this kind is published under.
    pub fn uri_prefix(self) -> &'static str {
        match self {
            Kind::Guide => "gore://guide/",
            Kind::Reference => "gore://reference/",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Guide => "guide",
            Kind::Reference => "reference",
        }
    }
}

/// One embedded Markdown page.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// Stable identifier used in `gore://<kind>/<slug>` and in the `gore_guide` tool. Unique across
    /// both kinds, so a slug alone always names exactly one page.
    pub slug: &'static str,
    /// Path under `docs/`, e.g. `guide/textures.md`.
    pub file: &'static str,
    pub kind: Kind,
    pub markdown: &'static str,
}

impl Page {
    pub fn uri(&self) -> String {
        format!("{}{}", self.kind.uri_prefix(), self.slug)
    }
}

impl Page {
    /// The first level-one heading, or the slug when a page has none.
    pub fn title(&self) -> &str {
        self.sections()
            .into_iter()
            .find(|section| section.level == 1)
            .map(|section| section.heading)
            .unwrap_or(self.slug)
    }

    pub fn sections(&self) -> Vec<Section<'static>> {
        sections(self.markdown)
    }

    pub fn section(&self, anchor: &str) -> Option<Section<'static>> {
        self.sections().into_iter().find(|section| section.anchor == anchor)
    }
}

/// Look up a page by slug. Exact match only — never by joining a path, which is what keeps
/// `gore://guide/../../secrets` from resolving to anything.
pub fn page(slug: &str) -> Option<&'static Page> {
    PAGES.iter().find(|page| page.slug == slug)
}

pub fn slugs() -> Vec<&'static str> {
    PAGES.iter().map(|page| page.slug).collect()
}

/// Every page of one kind, in reading order.
pub fn pages_of(kind: Kind) -> impl Iterator<Item = &'static Page> {
    PAGES.iter().filter(move |page| page.kind == kind)
}

/// The user guide alone — what ships in the release zip and what `gore guide html` renders.
pub fn guide_pages() -> impl Iterator<Item = &'static Page> {
    pages_of(Kind::Guide)
}

/// One heading and everything under it, up to the next heading of the same or higher level.
#[derive(Debug, Clone)]
pub struct Section<'a> {
    /// GitHub-style anchor, so `read(page, section)` accepts the same fragment the docs link to.
    pub anchor: String,
    pub heading: &'a str,
    pub level: u8,
    /// The section text, including its own heading line.
    pub body: &'a str,
}

/// Split a page at its headings.
///
/// Headings inside fenced code blocks are ignored. That is not a hypothetical: the guide is full of
/// PowerShell, where a comment starts with `#` at the beginning of a line, and treating those as
/// headings would shred half the pages into fragments.
pub fn sections(markdown: &str) -> Vec<Section<'_>> {
    let mut heads: Vec<(usize, usize, u8, &str)> = Vec::new();
    // The fence character and how many of it opened the block. Both matter: a closing fence carries
    // no info string and must be at least as long as the opener, so a page that documents Markdown
    // by wrapping ```toml in ```` stays one block instead of four.
    let mut fence: Option<(char, usize)> = None;
    let mut offset = 0usize;

    for line in markdown.split_inclusive('\n') {
        let start = offset;
        offset += line.len();

        let text = line.trim_end_matches(['\r', '\n']);
        let trimmed = text.trim_start();

        if let Some((character, opened_with)) = fence {
            let run = trimmed.chars().take_while(|candidate| *candidate == character).count();
            if run >= opened_with && trimmed[run..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(opened_with) = opening_fence(trimmed, '`') {
            fence = Some(('`', opened_with));
            continue;
        }
        if let Some(opened_with) = opening_fence(trimmed, '~') {
            fence = Some(('~', opened_with));
            continue;
        }

        // A heading starts at column zero; anything indented is code or a list continuation.
        if !text.starts_with('#') {
            continue;
        }
        let level = text.bytes().take_while(|byte| *byte == b'#').count();
        if !(1..=6).contains(&level) || text.as_bytes().get(level) != Some(&b' ') {
            continue;
        }
        heads.push((start, offset, level as u8, text[level + 1..].trim()));
    }

    let mut used: Vec<String> = Vec::new();
    let mut sections = Vec::with_capacity(heads.len());

    for (index, &(start, _, level, heading)) in heads.iter().enumerate() {
        let end = heads
            .iter()
            .skip(index + 1)
            .find(|(_, _, next_level, _)| *next_level <= level)
            .map(|(next_start, _, _, _)| *next_start)
            .unwrap_or(markdown.len());

        let mut anchor = anchor(heading);
        // GitHub disambiguates repeated headings by suffixing a counter; match that so anchors
        // copied out of the rendered docs keep working.
        if used.contains(&anchor) {
            let base = anchor.clone();
            let mut counter = 1;
            while used.contains(&anchor) {
                anchor = format!("{base}-{counter}");
                counter += 1;
            }
        }
        used.push(anchor.clone());

        sections.push(Section { anchor, heading, level, body: &markdown[start..end] });
    }

    sections
}

/// The length of a fence this line opens, if it opens one. Three or more of the same character.
fn opening_fence(trimmed: &str, character: char) -> Option<usize> {
    let run = trimmed.chars().take_while(|candidate| *candidate == character).count();
    (run >= 3).then_some(run)
}

/// The GitHub heading-anchor rule: lowercase, drop punctuation, spaces become hyphens.
pub fn anchor(heading: &str) -> String {
    let mut anchor = String::with_capacity(heading.len());
    for character in heading.chars() {
        if character.is_alphanumeric() {
            anchor.extend(character.to_lowercase());
        } else if character == ' ' {
            anchor.push('-');
        } else if character == '-' || character == '_' {
            anchor.push(character);
        }
    }
    anchor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_inside_a_fenced_block_is_not_a_heading() {
        // PowerShell comments look exactly like Markdown headings. Getting this wrong would split
        // most pages in the guide at arbitrary points inside their examples.
        let markdown = "# Real\n\n```powershell\n# not a heading\ngore config list\n```\n\n## Also real\n";
        let headings: Vec<&str> =
            sections(markdown).into_iter().map(|section| section.heading).collect();
        assert_eq!(headings, vec!["Real", "Also real"]);
    }

    #[test]
    fn an_info_string_does_not_close_a_fence() {
        // A page showing how to write a fenced block contains an inner ```toml. Treating that as
        // the close would end the block early and turn the rest of the example into headings.
        let markdown = concat!(
            "# Real

",
            "````
",
            "```toml
",
            "# not a heading
",
            "```
",
            "````

",
            "## Also real
",
        );
        let headings: Vec<&str> =
            sections(markdown).into_iter().map(|section| section.heading).collect();
        assert_eq!(headings, vec!["Real", "Also real"]);
    }

    #[test]
    fn tilde_fences_are_handled_too() {
        let markdown = "# Real\n\n~~~sh\n# nope\n~~~\n\n## Also real\n";
        assert_eq!(sections(markdown).len(), 2);
    }

    #[test]
    fn a_section_runs_until_the_next_heading_of_the_same_or_higher_level() {
        let markdown = "# One\na\n## Two\nb\n### Three\nc\n## Four\nd\n";
        let sections = sections(markdown);

        let two = sections.iter().find(|section| section.heading == "Two").unwrap();
        assert!(two.body.contains("b"));
        assert!(two.body.contains("Three"), "a subsection belongs to its parent");
        assert!(!two.body.contains("Four"), "a sibling does not");
    }

    #[test]
    fn a_section_body_includes_its_own_heading() {
        let sections = sections("# One\nbody\n");
        assert!(sections[0].body.starts_with("# One"));
    }

    #[test]
    fn anchors_follow_the_github_rule() {
        assert_eq!(anchor("Other helpers"), "other-helpers");
        assert_eq!(anchor("`mcp`"), "mcp");
        assert_eq!(anchor("Step 1: Extract!"), "step-1-extract");
        assert_eq!(anchor("snake_case-and-dash"), "snake_case-and-dash");
    }

    #[test]
    fn repeated_headings_get_distinct_anchors() {
        let sections = sections("## Notes\na\n## Notes\nb\n");
        assert_eq!(sections[0].anchor, "notes");
        assert_eq!(sections[1].anchor, "notes-1");
    }

    #[test]
    fn indented_hashes_are_not_headings() {
        let markdown = "# Real\n    # indented code\n";
        assert_eq!(sections(markdown).len(), 1);
    }

    #[test]
    fn every_embedded_page_parses_into_at_least_one_section_with_a_title() {
        for page in PAGES {
            let sections = page.sections();
            assert!(!sections.is_empty(), "{} has no headings at all", page.slug);
            assert!(!page.title().is_empty(), "{} has no title", page.slug);
        }
    }

    #[test]
    fn a_page_is_found_by_exact_slug_only() {
        assert!(page("textures").is_some());
        assert!(page("textures.md").is_none());
        assert!(page("../../secrets").is_none());
        assert!(page("TEXTURES").is_none());
    }

    #[test]
    fn anchors_are_unique_within_every_embedded_page() {
        for page in PAGES {
            let mut seen = std::collections::BTreeSet::new();
            for section in page.sections() {
                assert!(
                    seen.insert(section.anchor.clone()),
                    "{} has a duplicate anchor {}",
                    page.slug,
                    section.anchor
                );
            }
        }
    }
}
