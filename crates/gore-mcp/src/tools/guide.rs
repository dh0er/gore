//! `gore_guide` — search and read the embedded guide.
//!
//! The same pages are also exposed as `gore://guide/<slug>` resources, which is the more idiomatic
//! MCP shape for documents. This tool exists because resources are application-controlled: in many
//! clients only the *user* can attach one, and the model cannot go looking. A tool works everywhere
//! and can be called by the model on its own initiative, which is the case that matters when an
//! agent hits an unfamiliar command.

use serde_json::{json, Map, Value};

use crate::exec::to_error_result;
use crate::guide::{self, search, Kind};

pub const NAME: &str = "gore_guide";

/// Default number of search hits. Enough to choose from, small enough to read.
const DEFAULT_LIMIT: usize = 8;
const MAX_LIMIT: usize = 25;

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
         Reading a whole page is fine for short ones but some run to several hundred lines.\n\n\
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
        if !["action", "page", "section", "query", "limit"].contains(&key.as_str()) {
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
            text.push_str(&format!("\n{} — {} ({lines} lines)\n", page.slug, page.title()));
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
        "structuredContent": { "pages": structured },
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

    let Some(anchor) = arguments.get("section").and_then(Value::as_str) else {
        return json!({
            "content": [{ "type": "text", "text": page.markdown }],
            "structuredContent": {
                "page": page.slug,
                "kind": page.kind.label(),
                "title": page.title(),
            },
            "isError": false,
        });
    };

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

    json!({
        "content": [{ "type": "text", "text": section.body }],
        "structuredContent": {
            "page": page.slug,
            "kind": page.kind.label(),
            "anchor": section.anchor,
            "heading": section.heading,
        },
        "isError": false,
    })
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
            "structuredContent": { "hits": [] },
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
        "structuredContent": {
            "hits": hits
                .iter()
                .map(|hit| json!({
                    "page": hit.page,
                    "kind": kind_of(hit.page),
                    "anchor": hit.anchor,
                    "heading": hit.heading,
                    "score": hit.score,
                    "snippet": hit.snippet,
                }))
                .collect::<Vec<_>>(),
        },
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
        assert_eq!(
            result["structuredContent"]["pages"].as_array().unwrap().len(),
            guide::PAGES.len()
        );
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
        assert!(!result["structuredContent"]["hits"].as_array().unwrap().is_empty());
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
        let two = call_with(json!({ "action": "search", "query": "mod", "limit": 2 }));
        assert_eq!(two["structuredContent"]["hits"].as_array().unwrap().len(), 2);

        let huge = call_with(json!({ "action": "search", "query": "mod", "limit": 9999 }));
        assert!(huge["structuredContent"]["hits"].as_array().unwrap().len() <= MAX_LIMIT);
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
