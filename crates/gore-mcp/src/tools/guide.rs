//! `gore_guide` — search and read the embedded guide.
//!
//! The same pages are also exposed as `gore://guide/<slug>` resources, which is the more idiomatic
//! MCP shape for documents. This tool exists because resources are application-controlled: in many
//! clients only the *user* can attach one, and the model cannot go looking. A tool works everywhere
//! and can be called by the model on its own initiative, which is the case that matters when an
//! agent hits an unfamiliar command.

use serde_json::{json, Map, Value};

use crate::exec::to_error_result;
use crate::guide::{self, search, Kind, Page};

pub const NAME: &str = "gore_guide";

/// Default number of search hits. Enough to choose from, small enough to read.
const DEFAULT_LIMIT: usize = 8;
const MAX_LIMIT: usize = 25;

/// How much of a page one `read` hands back before it is split into parts.
///
/// The four reference pages run past 20 000 characters, and a client that clips a tool result
/// clips it silently: a real session took seven reads back cut, one of them 20 811 characters
/// short, with no argument that could ask for the rest. Splitting here means the server decides
/// where the cut falls — on a heading — and can say what is on the other side of it.
///
/// 12 000 leaves sixteen of the twenty-four embedded pages whole and splits the largest into
/// four.
const PART_BUDGET_CHARS: usize = 12_000;

/// At most this many anchors are listed per part before the header says "and N more". A header
/// long enough to need scrolling is a second problem, not a fix for the first.
const ANCHORS_LISTED: usize = 8;

pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "GORE guide",
        "description": description(),
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "read", "search"],
                    "description": "list: every page with its sections. read: one page, or one \
                                    section of it. search: rank sections against a query.",
                },
                "page": {
                    "type": "string",
                    "enum": guide::slugs(),
                    "description": "Page slug. Required for `read`.",
                },
                "section": {
                    "type": "string",
                    "description": "Anchor of one section within `page`, as reported by `list` or \
                                    `search`. Omit to read the whole page.",
                },
                "part": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Which part of a long read to return. A read too long for one \
                                    result is split at heading boundaries; every part names the \
                                    sections in the other parts, so nothing has to be guessed at. \
                                    Defaults to part 1.",
                },
                "query": {
                    "type": "string",
                    "description": "Search terms. Required for `search`.",
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_LIMIT,
                    "description": "Maximum number of search hits.",
                },
            },
            "required": ["action"],
            "additionalProperties": false,
        },
        "annotations": {
            "title": "GORE guide",
            "readOnlyHint": true,
            "openWorldHint": false,
        },
    })
}

fn description() -> String {
    let guide_slugs: Vec<&str> = guide::pages_of(Kind::Guide).map(|page| page.slug).collect();
    let reference_slugs: Vec<&str> =
        guide::pages_of(Kind::Reference).map(|page| page.slug).collect();

    format!(
        "Search and read the GORE documentation.\n\n\
         Read it before running an unfamiliar command. The tool schemas list flags; this explains \
         which command to reach for, in what order, and what breaks if a step is skipped.\n\n\
         Start with `search`; it ranks individual sections, so the follow-up `read` stays small. \
         Reading a whole page is fine for short ones but some run to several hundred lines; those \
         come back in parts, each naming what the other parts hold.\n\n\
         Guide ({} pages) — how to mod the game: {}.\n\n\
         Reference ({} pages) — the contracts and invariants behind those commands. Reach for \
         these when a command refuses something and the guide does not say why: {}.",
        guide_slugs.len(),
        guide_slugs.join(", "),
        reference_slugs.len(),
        reference_slugs.join(", "),
    )
}

pub fn call(arguments: &Map<String, Value>) -> Value {
    for key in arguments.keys() {
        if !["action", "page", "section", "query", "limit", "part"].contains(&key.as_str()) {
            return to_error_result(format!("`{key}` is not an argument of {NAME}."));
        }
    }

    match arguments.get("action").and_then(Value::as_str) {
        Some("list") => list(),
        Some("read") => read(arguments),
        Some("search") => run_search(arguments),
        Some(other) => to_error_result(format!(
            "`{other}` is not a valid action. Use list, read or search."
        )),
        None => to_error_result("`action` is required and must be one of list, read, search."),
    }
}

fn list() -> Value {
    let mut text = String::from("GORE documentation. Use read(page, section) or search(query).\n");
    let mut structured = Vec::with_capacity(guide::PAGES.len());

    // Grouped, because the two bodies answer different questions and mixing them would leave the
    // model guessing which pages are instructions and which are contracts.
    for (kind, heading) in [
        (Kind::Guide, "GUIDE — how to mod the game"),
        (Kind::Reference, "REFERENCE — contracts and invariants behind the commands"),
    ] {
        text.push_str(&format!("\n== {heading} ==\n"));

        for page in guide::pages_of(kind) {
            let sections = page.sections();
            let lines = page.markdown.lines().count();
            // The part count is here so a long read is never a surprise: a model that can see the
            // page comes in three pieces can decide to ask for one section instead.
            let parts = parts_of(page).len();
            let size = match parts {
                1 => format!("{lines} lines"),
                many => format!("{lines} lines, {many} parts"),
            };
            text.push_str(&format!("\n{} — {} ({size})\n", page.slug, page.title()));
            for section in &sections {
                // Only the outline: listing every fourth-level heading would make `list` as long
                // as the guide.
                if section.level <= 2 {
                    text.push_str(&format!("    #{} — {}\n", section.anchor, section.heading));
                }
            }
            structured.push(json!({
                "slug": page.slug,
                "kind": kind.label(),
                "title": page.title(),
                "lines": lines,
                "parts": parts,
                "sections": sections
                    .iter()
                    .map(|section| json!({
                        "anchor": section.anchor,
                        "heading": section.heading,
                        "level": section.level,
                    }))
                    .collect::<Vec<_>>(),
            }));
        }
    }

    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false,
    })
}

fn read(arguments: &Map<String, Value>) -> Value {
    let Some(slug) = arguments.get("page").and_then(Value::as_str) else {
        return to_error_result(format!(
            "`read` requires `page`. Available pages: {}.",
            guide::slugs().join(", ")
        ));
    };
    let Some(page) = guide::page(slug) else {
        return to_error_result(format!(
            "There is no guide page `{slug}`. Available pages: {}.",
            guide::slugs().join(", ")
        ));
    };

    let wanted_part = match arguments.get("part") {
        None => 1usize,
        Some(value) => match value.as_u64() {
            Some(part) if part >= 1 => part as usize,
            _ => return to_error_result("`part` must be a whole number, 1 or greater."),
        },
    };

    let (parts, what, wanted_section) = match arguments.get("section").and_then(Value::as_str) {
        None => (parts_of(page), format!("`{slug}`"), None),
        Some(anchor) => {
            // Accept `#anchor` as well; that is how the fragment appears in the docs' own links.
            let wanted = anchor.trim_start_matches('#');
            let Some(section) = page.section(wanted) else {
                let sections = page.sections();
                let available: Vec<&str> =
                    sections.iter().map(|section| section.anchor.as_str()).collect();
                return to_error_result(format!(
                    "`{slug}` has no section `{wanted}`. Sections: {}.",
                    available.join(", ")
                ));
            };
            (
                parts_of_section(page, &section),
                format!("`{slug}#{wanted}`"),
                Some(wanted.to_string()),
            )
        }
    };

    let Some(part) = parts.get(wanted_part - 1) else {
        return to_error_result(format!(
            "{what} has {} part(s), so there is no part {wanted_part}.",
            parts.len()
        ));
    };

    // A read that fits is returned exactly as it is written. Prefixing every short page with
    // bookkeeping would tax the common case for a problem it does not have.
    let text = if parts.len() == 1 {
        part.text.to_string()
    } else {
        let section = wanted_section.as_deref();
        format!(
            "{}\n\n{}\n\n{}",
            header(slug, section, &what, &parts, wanted_part),
            part.text,
            footer(slug, section, &parts, wanted_part)
        )
    };

    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false,
    })
}

/// One piece of a split read: the text, and the headings that begin inside it.
struct Part {
    text: &'static str,
    /// `(anchor, heading)` for every heading this part starts with or contains.
    headings: Vec<(String, String)>,
}

impl Part {
    /// `#a, #b, #c` — capped, because a header nobody reads to the end helps nobody.
    fn anchors(&self) -> String {
        let shown: Vec<String> = self
            .headings
            .iter()
            .take(ANCHORS_LISTED)
            .map(|(anchor, _)| format!("#{anchor}"))
            .collect();
        match self.headings.len().saturating_sub(shown.len()) {
            0 if shown.is_empty() => "(no heading of its own)".to_string(),
            0 => shown.join(", "),
            more => format!("{}, and {more} more", shown.join(", ")),
        }
    }
}

/// What comes before the text: where this part sits, and what is in all the others.
///
/// The whole point of the split. A truncated read that says only "there is more" leaves the model
/// guessing which section it wanted, and the cheapest way to guess is to read the page again — the
/// loop this replaces. Naming every part's sections makes the follow-up one call, whether that call
/// is the next part or the one section that was actually wanted.
fn header(
    slug: &str,
    section: Option<&str>,
    what: &str,
    parts: &[Part],
    current: usize,
) -> String {
    let mut text = format!("[{what} is {} parts; this is part {current}.]\n", parts.len());
    for (index, part) in parts.iter().enumerate() {
        let number = index + 1;
        let chars = part.text.chars().count();
        if number == current {
            text.push_str(&format!("part {number} (this one, {chars} chars): {}\n", part.anchors()));
        } else {
            text.push_str(&format!(
                "part {number} ({chars} chars): {} — read with {}\n",
                part.anchors(),
                read_call(slug, section, number)
            ));
        }
    }
    text.push_str(
        "Any one section can also be read on its own with \
         gore_guide{action:\"read\", page:\"…\", section:\"<anchor>\"}.",
    );
    text
}

fn footer(slug: &str, section: Option<&str>, parts: &[Part], current: usize) -> String {
    match parts.get(current) {
        Some(next) => format!(
            "[end of part {current} of {}. Next: {} — {}]",
            parts.len(),
            next.anchors(),
            read_call(slug, section, current + 1)
        ),
        None => format!("[end of part {current} of {}, and of {slug}.]", parts.len()),
    }
}

/// The call that fetches another part of exactly this read. A section read continues within the
/// section, not into the rest of the page.
fn read_call(slug: &str, section: Option<&str>, part: usize) -> String {
    match section {
        Some(anchor) => format!(
            "gore_guide{{action:\"read\", page:\"{slug}\", section:\"{anchor}\", part:{part}}}"
        ),
        None => format!("gore_guide{{action:\"read\", page:\"{slug}\", part:{part}}}"),
    }
}

/// Split a whole page at its headings.
fn parts_of(page: &'static Page) -> Vec<Part> {
    let cuts: Vec<(usize, String, String)> = page
        .sections()
        .into_iter()
        .map(|section| (section.start, section.anchor, section.heading.to_string()))
        .collect();
    paginate(page.markdown, &cuts)
}

/// Split one section at the headings nested inside it.
///
/// Today no section in the guide comes close to the budget, so this always returns one part. It
/// exists so that the day one does, a section read behaves like a page read instead of quietly
/// handing back more than the client will show.
fn parts_of_section(page: &'static Page, section: &guide::Section<'static>) -> Vec<Part> {
    let end = section.start + section.body.len();
    let cuts: Vec<(usize, String, String)> = page
        .sections()
        .into_iter()
        .filter(|candidate| candidate.start >= section.start && candidate.start < end)
        .map(|candidate| {
            (candidate.start - section.start, candidate.anchor, candidate.heading.to_string())
        })
        .collect();
    paginate(section.body, &cuts)
}

/// Pack the pieces between `cuts` into parts no larger than the budget.
///
/// `cuts` are byte offsets into `text` where a heading begins, in document order. Every heading
/// counts, not just the top-level ones: cutting `studio-authoring` at its `##` headings alone would
/// leave a 19 000-character piece, which is most of the problem still unsolved.
fn paginate(text: &'static str, cuts: &[(usize, String, String)]) -> Vec<Part> {
    let mut parts: Vec<Part> = Vec::new();
    let mut start = 0usize;
    let mut headings: Vec<(String, String)> = Vec::new();

    for (index, (offset, anchor, heading)) in cuts.iter().enumerate() {
        // Anything before the first heading belongs to the first part; there is nothing to cut.
        if *offset <= start {
            headings.push((anchor.clone(), heading.clone()));
            continue;
        }

        let next_end = cuts.get(index + 1).map_or(text.len(), |(next, _, _)| *next);
        let would_be = text[start..next_end].chars().count();
        if would_be > PART_BUDGET_CHARS && !headings.is_empty() {
            parts.push(Part { text: &text[start..*offset], headings: std::mem::take(&mut headings) });
            start = *offset;
        }
        headings.push((anchor.clone(), heading.clone()));
    }

    // The tail, which is also the whole text when nothing was ever over budget. A single piece
    // larger than the budget is emitted whole rather than cut mid-sentence: the header says how
    // large it is, and a heading is the only place this splits.
    parts.push(Part { text: &text[start..], headings });
    parts
}

/// The kind label for a slug the search returned. Search ranks sections across both bodies, and a
/// hit is worth much less if the model cannot tell instructions from contract.
fn kind_of(slug: &str) -> &'static str {
    guide::page(slug).map(|page| page.kind.label()).unwrap_or("guide")
}

fn run_search(arguments: &Map<String, Value>) -> Value {
    let Some(query) = arguments.get("query").and_then(Value::as_str) else {
        return to_error_result("`search` requires `query`.");
    };

    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|limit| (limit as usize).clamp(1, MAX_LIMIT))
        .unwrap_or(DEFAULT_LIMIT);

    let hits = search::search(query, limit);
    if hits.is_empty() {
        return json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "Nothing in the guide matches {query:?}. Try fewer or more general words, or \
                     call gore_guide with action \"list\" to see what the guide covers.",
                ),
            }],
            "isError": false,
        });
    }

    let mut text = format!("{} sections match {query:?}:\n", hits.len());
    for hit in &hits {
        text.push_str(&format!(
            "\n[{}] {}#{} — {}\n    {}\n    read with: gore_guide{{action:\"read\", page:\"{}\", section:\"{}\"}}\n",
            kind_of(hit.page), hit.page, hit.anchor, hit.heading, hit.snippet, hit.page, hit.anchor
        ));
    }

    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_with(arguments: Value) -> Value {
        let Value::Object(map) = arguments else { panic!("test arguments must be an object") };
        call(&map)
    }

    fn text_of(result: &Value) -> String {
        result["content"][0]["text"].as_str().expect("a text block").to_string()
    }

    #[test]
    fn the_definition_names_every_page_so_a_client_can_validate_the_slug() {
        let definition = definition();
        let listed = definition["inputSchema"]["properties"]["page"]["enum"]
            .as_array()
            .expect("an enum of slugs")
            .len();
        assert_eq!(listed, guide::PAGES.len());
        assert_eq!(definition["annotations"]["readOnlyHint"], json!(true));
    }

    #[test]
    fn list_reports_every_page_with_its_outline() {
        let result = call_with(json!({ "action": "list" }));
        assert_eq!(result["isError"], json!(false));

        let text = text_of(&result);
        for page in guide::PAGES {
            assert!(text.contains(page.slug), "{} missing from the listing", page.slug);
        }
        // One line per page in the listing itself; there is no second channel to check against.
        for page in guide::PAGES {
            assert!(text.contains(page.title()), "{} has no title line", page.slug);
        }
    }

    #[test]
    fn read_returns_a_whole_page() {
        let result = call_with(json!({ "action": "read", "page": "textures" }));
        assert_eq!(result["isError"], json!(false));
        assert_eq!(text_of(&result), guide::page("textures").unwrap().markdown);
    }

    #[test]
    fn read_can_narrow_to_one_section() {
        let page = guide::page("cli-reference").unwrap();
        let section = &page.sections()[1];

        let result = call_with(json!({
            "action": "read",
            "page": "cli-reference",
            "section": section.anchor,
        }));
        let text = text_of(&result);

        assert!(text.contains(section.heading));
        assert!(text.len() < page.markdown.len(), "a section should be smaller than its page");
    }

    #[test]
    fn a_leading_hash_on_the_section_is_accepted() {
        let anchor = guide::page("cli-reference").unwrap().sections()[1].anchor.clone();
        let with_hash = call_with(json!({
            "action": "read",
            "page": "cli-reference",
            "section": format!("#{anchor}"),
        }));
        let without = call_with(json!({
            "action": "read",
            "page": "cli-reference",
            "section": anchor,
        }));
        assert_eq!(with_hash, without);
    }

    /// The first page long enough to be split, so no test has to name one and then go stale when
    /// the docs are edited.
    fn a_split_page() -> &'static guide::Page {
        guide::PAGES
            .iter()
            .find(|page| parts_of(page).len() > 1)
            .expect("the reference pages are well past the budget")
    }

    #[test]
    fn a_long_page_arrives_in_parts_that_name_what_the_others_hold() {
        // The finding this exists for: seven reads in one session came back clipped by the client,
        // one of them 20 811 characters short, and the only way on was a section read — which needs
        // an anchor, which needs the search that had just failed. A part that names the other
        // parts' sections makes the next move one call and not a guess.
        let page = a_split_page();
        let parts = parts_of(page);

        let text = text_of(&call_with(json!({ "action": "read", "page": page.slug })));
        let opening = format!("[`{}` is {} parts; this is part 1.]", page.slug, parts.len());
        assert!(text.starts_with(&opening), "{text:.400}");
        assert!(
            text.contains(&format!("gore_guide{{action:\"read\", page:\"{}\", part:2}}", page.slug)),
            "part 1 must say how to get part 2: {text:.400}"
        );
        // Every anchor of every later part is named up front, not just the next one's.
        for part in &parts[1..] {
            for (anchor, _) in part.headings.iter().take(ANCHORS_LISTED) {
                assert!(text.contains(&format!("#{anchor}")), "part 1 never mentions #{anchor}");
            }
        }
    }

    #[test]
    fn the_parts_of_a_page_are_the_whole_page_and_nothing_but() {
        // A split that quietly dropped a paragraph would be worse than the truncation it replaces:
        // truncation at least announces itself.
        for page in guide::PAGES {
            let rejoined: String =
                parts_of(page).iter().map(|part| part.text).collect::<Vec<_>>().concat();
            assert_eq!(rejoined, page.markdown, "{} does not survive being split", page.slug);
        }
    }

    #[test]
    fn no_part_runs_past_the_budget_unless_one_section_does() {
        // The cut points are every heading, at any level. Cutting only at `##` would leave
        // `studio-authoring` with a 19 000-character part, which is the same problem with extra
        // steps.
        for page in guide::PAGES {
            let parts = parts_of(page);
            for (index, part) in parts.iter().enumerate() {
                let size = part.text.chars().count();
                assert!(
                    size <= PART_BUDGET_CHARS || part.headings.len() <= 1,
                    "{} part {} is {size} chars across {} headings",
                    page.slug,
                    index + 1,
                    part.headings.len()
                );
            }
        }
    }

    #[test]
    fn a_part_beyond_the_last_one_says_how_many_there_are() {
        let page = a_split_page();
        let result = call_with(json!({ "action": "read", "page": page.slug, "part": 99 }));
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result).contains("part(s)"), "{}", text_of(&result));

        let refused = call_with(json!({ "action": "read", "page": page.slug, "part": 0 }));
        assert_eq!(refused["isError"], json!(true));
    }

    #[test]
    fn a_section_read_continues_within_its_own_section() {
        // No section is near the budget today, so this is about the shape of the pointer rather
        // than about a page that needs it: a `part` offered on a section read must not silently
        // walk off into the rest of the page.
        let page = a_split_page();
        let anchor = page.sections()[1].anchor.clone();
        let parts = parts_of_section(page, &page.section(&anchor).unwrap());
        assert_eq!(parts.len(), 1, "the fixture assumes sections fit; {anchor} no longer does");
        assert_eq!(
            read_call(page.slug, Some(&anchor), 2),
            format!(
                "gore_guide{{action:\"read\", page:\"{}\", section:\"{anchor}\", part:2}}",
                page.slug
            )
        );
    }

    #[test]
    fn the_listing_says_which_pages_come_in_parts() {
        // So the choice between a page read and a section read can be made before the read, which
        // is the only point at which it is cheap.
        let text = text_of(&call_with(json!({ "action": "list" })));
        let page = a_split_page();
        let parts = parts_of(page).len();
        assert!(
            text.contains(&format!("{} parts)", parts)),
            "no page is marked as split: {text:.600}"
        );
    }

    #[test]
    fn an_unknown_page_lists_the_real_ones() {
        let result = call_with(json!({ "action": "read", "page": "nope" }));
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result).contains("textures"));
    }

    #[test]
    fn an_unknown_section_lists_the_real_ones() {
        let result =
            call_with(json!({ "action": "read", "page": "textures", "section": "nope" }));
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result).contains("Sections:"));
    }

    #[test]
    fn a_path_traversal_slug_is_simply_not_a_page() {
        let result = call_with(json!({ "action": "read", "page": "../../../etc/passwd" }));
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result).contains("no guide page"));
    }

    #[test]
    fn search_returns_hits_that_say_how_to_read_them() {
        let result = call_with(json!({ "action": "search", "query": "replace a texture" }));
        assert_eq!(result["isError"], json!(false));

        let text = text_of(&result);
        assert!(text.contains("gore_guide{action:\"read\""), "{text}");
        assert!(text.contains("textures"), "the top hit belongs in the text: {text}");
    }

    #[test]
    fn a_search_that_finds_nothing_says_what_to_try_instead() {
        let result = call_with(json!({ "action": "search", "query": "zzzznotaword" }));
        // Not an error: an empty result set is a valid answer, and telling the model it failed
        // would invite a pointless retry.
        assert_eq!(result["isError"], json!(false));
        assert!(text_of(&result).contains("list"));
    }

    #[test]
    fn the_limit_is_honoured_and_clamped() {
        // Counted from the rendered text, because that is what the model is handed. Each hit
        // contributes exactly one "read with:" line.
        let hits_in = |result: &Value| text_of(result).matches("read with:").count();

        let two = call_with(json!({ "action": "search", "query": "mod", "limit": 2 }));
        assert_eq!(hits_in(&two), 2);
        assert!(text_of(&two).starts_with("2 sections match"), "{}", text_of(&two));

        let huge = call_with(json!({ "action": "search", "query": "mod", "limit": 9999 }));
        assert!(hits_in(&huge) <= MAX_LIMIT, "{}", hits_in(&huge));
    }

    #[test]
    fn a_missing_or_unknown_action_explains_the_valid_ones() {
        assert!(text_of(&call_with(json!({}))).contains("list, read, search"));
        assert!(text_of(&call_with(json!({ "action": "delete" }))).contains("list, read or search"));
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let result = call_with(json!({ "action": "list", "pages": 1 }));
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result).contains("`pages`"));
    }
}
