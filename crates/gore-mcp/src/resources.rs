//! The documentation as MCP resources: `gore://guide/<page>` and `gore://reference/<page>`.
//!
//! Resources are the idiomatic MCP shape for documents, and they are how a *person* attaches a
//! guide page to a conversation in clients that support it. They do not replace the `gore_guide`
//! tool: resources are application-controlled, so in many clients the model cannot discover or
//! fetch one on its own. The two views share the same embedded pages, so there is nothing to keep
//! in sync.

use serde_json::{json, Value};

use crate::guide::{self, Kind};

/// How long a description may get before it stops being a description.
const MAX_DESCRIPTION_CHARS: usize = 180;

pub fn list() -> Vec<Value> {
    guide::PAGES
        .iter()
        .map(|page| {
            json!({
                "uri": page.uri(),
                "name": page.slug,
                "title": page.title(),
                "description": summarize(page.markdown),
                "mimeType": "text/markdown",
                "size": page.markdown.len(),
            })
        })
        .collect()
}

pub fn templates() -> Vec<Value> {
    [
        (Kind::Guide, "gore-guide-page", "GORE guide page", "the GORE modding guide"),
        (
            Kind::Reference,
            "gore-reference-page",
            "GORE reference page",
            "the GORE technical reference — contracts and invariants behind the guide",
        ),
    ]
    .into_iter()
    .map(|(kind, name, title, what)| {
        let slugs: Vec<&str> = guide::pages_of(kind).map(|page| page.slug).collect();
        json!({
            "uriTemplate": format!("{}{{page}}", kind.uri_prefix()),
            "name": name,
            "title": title,
            "description": format!("One page of {what}. Valid pages: {}.", slugs.join(", ")),
            "mimeType": "text/markdown",
        })
    })
    .collect()
}

/// Resolve a resource URI.
///
/// The slug is compared for exact equality against the embedded page list — it is never joined onto
/// a filesystem path. That is what makes `gore://guide/../../../etc/passwd` simply an unknown page
/// rather than a traversal: there is no path to traverse. The prefix must also match the page's own
/// kind, so a reference page cannot be reached through the guide namespace or the other way round.
pub fn read(uri: &str) -> Option<Value> {
    let page = [Kind::Guide, Kind::Reference].into_iter().find_map(|kind| {
        let slug = uri.strip_prefix(kind.uri_prefix())?;
        guide::page(slug).filter(|page| page.kind == kind)
    })?;

    Some(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/markdown",
            "text": page.markdown,
        }],
    }))
}

/// The first real sentence of a page, for the resource listing.
fn summarize(markdown: &str) -> String {
    let paragraph = markdown
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with('|')
                && !line.starts_with("```")
                && !line.starts_with('>')
        })
        .unwrap_or_default();

    if paragraph.chars().count() <= MAX_DESCRIPTION_CHARS {
        return paragraph.to_string();
    }
    let cut: String = paragraph.chars().take(MAX_DESCRIPTION_CHARS).collect();
    // Prefer to end on a word rather than mid-token.
    match cut.rfind(' ') {
        Some(space) => format!("{}…", &cut[..space]),
        None => format!("{cut}…"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_is_listed_with_the_fields_a_client_needs() {
        let listed = list();
        assert_eq!(listed.len(), guide::PAGES.len());

        for resource in &listed {
            let uri = resource["uri"].as_str().unwrap();
            assert!(
                uri.starts_with("gore://guide/") || uri.starts_with("gore://reference/"),
                "{uri} is in neither namespace"
            );
            assert_eq!(resource["mimeType"], "text/markdown");
            assert!(resource["name"].as_str().is_some_and(|name| !name.is_empty()));
            assert!(resource["title"].as_str().is_some_and(|title| !title.is_empty()));
            assert!(resource["size"].as_u64().unwrap() > 0);
        }
    }

    #[test]
    fn descriptions_are_short_and_free_of_markdown_furniture() {
        for resource in list() {
            let description = resource["description"].as_str().unwrap();
            assert!(
                description.chars().count() <= MAX_DESCRIPTION_CHARS + 1,
                "{description:?} is too long"
            );
            assert!(!description.starts_with('#'), "{description:?}");
            assert!(!description.starts_with('|'), "{description:?}");
        }
    }

    #[test]
    fn one_template_covers_each_namespace() {
        let templates = templates();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0]["uriTemplate"], "gore://guide/{page}");
        assert_eq!(templates[1]["uriTemplate"], "gore://reference/{page}");
    }

    #[test]
    fn a_page_is_only_reachable_through_its_own_namespace() {
        // Otherwise the two bodies of documentation would be one namespace wearing two names, and
        // `gore://guide/<something>` would stop meaning "this is user documentation".
        let reference = guide::pages_of(Kind::Reference).next().expect("a reference page");
        assert!(read(&format!("gore://reference/{}", reference.slug)).is_some());
        assert!(read(&format!("gore://guide/{}", reference.slug)).is_none());

        let user = guide::pages_of(Kind::Guide).next().expect("a guide page");
        assert!(read(&format!("gore://guide/{}", user.slug)).is_some());
        assert!(read(&format!("gore://reference/{}", user.slug)).is_none());
    }

    #[test]
    fn a_known_uri_reads_the_whole_page() {
        let contents = read("gore://guide/textures").expect("textures resolves");
        assert_eq!(contents["contents"][0]["uri"], "gore://guide/textures");
        assert_eq!(contents["contents"][0]["mimeType"], "text/markdown");
        assert_eq!(
            contents["contents"][0]["text"],
            guide::page("textures").unwrap().markdown
        );
    }

    #[test]
    fn every_listed_resource_can_actually_be_read() {
        for resource in list() {
            let uri = resource["uri"].as_str().unwrap();
            assert!(read(uri).is_some(), "{uri} is listed but does not resolve");
        }
    }

    #[test]
    fn a_traversal_attempt_is_just_an_unknown_page() {
        assert!(read("gore://guide/../../../etc/passwd").is_none());
        assert!(read("gore://guide/..%2F..%2Fsecrets").is_none());
        assert!(read("gore://guide/").is_none());
    }

    #[test]
    fn a_foreign_scheme_is_rejected() {
        assert!(read("file:///etc/passwd").is_none());
        assert!(read("gore://other/textures").is_none());
        assert!(read("https://example.com").is_none());
    }
}
