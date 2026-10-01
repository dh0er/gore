//! `gore guide` — the embedded guide, rendered for a browser or ranked for a shell.
//!
//! # `html`
//!
//! Render the embedded guide into one self-contained HTML file.
//!
//! The release zip ships the guide as Markdown beside `gore.exe`, which is fine for `grep` but poor
//! for actually reading: Windows has no default handler for `.md`, so a double-click lands in
//! Notepad, and the guide is table-heavy enough that raw pipe tables are close to unreadable there.
//! (The MCP server does not read those files either — it serves the copy `include_str!`d into this
//! binary, which is why an edit to the staged Markdown reaches a human and no agent.)
//!
//! What this produces is deliberately *one file*, with every page, its stylesheet and its script
//! inlined. A thinner wrapper that fetched the `.md` files at runtime would be less code, but it
//! cannot work: opened over `file://` a browser treats the page's origin as opaque and blocks every
//! `fetch`/XHR against the neighbouring files, so the reader would get a blank page.
//!
//! The source is the same `include_str!`-embedded guide the MCP server serves
//! (`gore_mcp::guide`), so the rendered document cannot drift from the pages in `docs/guide/`.
//!
//! # `search`
//!
//! Run the MCP server's own section ranker from a shell.
//!
//! The primer tells an agent to start every task with `gore_guide` `search`, which made that
//! ranking the single most-used thing in the server — and until this subcommand existed there was
//! no way to exercise it without speaking JSON-RPC to a running server. A ranking nobody can watch
//! is a ranking nobody fixes: the two defects this command was added alongside had both been live
//! for months and were found by reading the code, not by running it.
//!
//! It calls `gore_mcp::guide::search` directly, so what a shell sees is what an agent gets.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gore_mcp::guide::{self, Kind, Page};
use pulldown_cmark::{html, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Default for `--repo-ref`. `build.py` passes the exact commit instead, so a shipped link keeps
/// resolving to the tree the guide was written against.
pub const DEFAULT_REPO_REF: &str = "main";

const REPO_WEB_BASE: &str = "https://github.com/dh0er/gore";

/// Where the guide pages live, relative to the repo root. Outbound `../` links are resolved against
/// this, exactly as `stage_docs` in `build.py` does for the Markdown copies.
const GUIDE_DIR: &str = "docs/guide";

/// Default for `gore guide search --limit`. The same number the MCP tool defaults to, so the two
/// answer a query with the same list.
pub const DEFAULT_SEARCH_LIMIT: usize = 8;

/// Ceiling for `--limit`, clamped rather than refused — also the MCP tool's, for the same reason:
/// past this the list is longer than the page it is pointing at.
pub const MAX_SEARCH_LIMIT: usize = 25;

pub fn html_file(out: PathBuf, repo_ref: &str) -> Result<()> {
    let document = render(repo_ref);

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(&out, &document).with_context(|| format!("writing {}", out.display()))?;

    println!(
        "wrote {} ({} pages, {} KiB)",
        out.display(),
        guide::guide_pages().count(),
        document.len().div_ceil(1024)
    );
    Ok(())
}

/// Rank guide and reference sections against a query and print them, best first.
///
/// `terms` are the bare words as clap collected them; they are joined because the ranker takes one
/// string and splits it itself. Accepting several is what lets a query be typed without quoting.
pub fn search(terms: &[String], limit: usize) -> Result<()> {
    let query = terms.join(" ");
    let hits = guide::search::search(&query, limit.clamp(1, MAX_SEARCH_LIMIT));

    println!("{}", search_listing(&query, &hits));
    Ok(())
}

/// The human-readable listing. Separate from printing it so a test can read what a user reads.
fn search_listing(query: &str, hits: &[guide::search::Hit]) -> String {
    // Not an error and not a bail: an empty result set is a real answer to a real question, and
    // the useful thing to say is what to try instead.
    if hits.is_empty() {
        return format!(
            "nothing in the guide matches {query:?}\n\
             try fewer or more general words, or 'gore guide html' to browse the whole thing"
        );
    }

    let mut text = format!("{} section(s) match {query:?}, best first:", hits.len());
    for hit in hits {
        let page = guide::page(hit.page);
        let kind = page.map(|page| page.kind.label()).unwrap_or("guide");
        let file = page.map(|page| format!("docs/{}", page.file)).unwrap_or_default();
        // The score is printed because this command exists to debug the ranking, and a list
        // without the numbers cannot answer why one hit beat another.
        text.push_str(&format!(
            "\n\n[{}] {}#{} — {}\n     {kind} · {file}\n     {}",
            hit.score, hit.page, hit.anchor, hit.heading, hit.snippet
        ));
    }
    text
}

/// One entry in the navigation sidebar.
struct Entry {
    level: u8,
    text: String,
    /// The document-wide `id`, already namespaced by page.
    anchor: String,
}

/// Build the whole document.
pub fn render(repo_ref: &str) -> String {
    let mut nav = String::new();
    let mut body = String::new();

    // Only the user guide. docs/reference/ is embedded in the binary for the MCP server, but it
    // is contracts rather than instructions and does not belong in a document a reader browses.
    for page in guide::guide_pages() {
        let (page_html, entries) = render_page(page, repo_ref);

        let title = entries
            .iter()
            .find(|entry| entry.level == 1)
            .map(|entry| entry.text.clone())
            .unwrap_or_else(|| page.title().to_string());

        // Only level-2 headings are listed, and they start collapsed. Expanded, 22 pages come to
        // roughly 200 entries — a wall nobody scans. Collapsed, the sidebar is the table of
        // contents it should have been, and the filter or the scroll-spy opens what is relevant.
        let subsections: Vec<&Entry> = entries.iter().filter(|entry| entry.level == 2).collect();

        if subsections.is_empty() {
            nav.push_str(&format!(
                "<li class=\"nav-page\"><a class=\"nav-leaf\" href=\"#{slug}\">{title}</a></li>",
                slug = escape(page.slug),
                title = escape(&title),
            ));
        } else {
            // The page title is the summary's own text, not a nested link. A link inside a
            // <summary> gives one row two click targets with two different outcomes under a
            // single hover — the script navigates on expand instead, so a row does one thing.
            nav.push_str(&format!(
                "<li class=\"nav-page\"><details><summary data-target=\"{slug}\">{title}\
                 </summary><ul>",
                slug = escape(page.slug),
                title = escape(&title),
            ));
            for entry in subsections {
                nav.push_str(&format!(
                    "<li><a href=\"#{anchor}\">{text}</a></li>",
                    anchor = escape(&entry.anchor),
                    text = escape(&entry.text),
                ));
            }
            nav.push_str("</ul></details></li>");
        }

        body.push_str(&format!(
            "<section class=\"page\" id=\"{slug}\">{page_html}</section>",
            slug = escape(page.slug),
        ));
    }

    shell(&nav, &body)
}

/// Render one page, returning its HTML and the headings it contributed to the sidebar.
fn render_page(page: &Page, repo_ref: &str) -> (String, Vec<Entry>) {
    let mut options = Options::empty();
    // The guide is full of tables; without this they render as literal pipes.
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut events: Vec<Event> = Parser::new_ext(page.markdown, options).collect();

    // Pass one: name every heading. The `id` has to be set on the opening event, which has already
    // gone by the time the heading's text arrives, so the text is collected first.
    let mut entries: Vec<Entry> = Vec::new();
    let mut used: Vec<String> = Vec::new();
    let mut heading_ids: Vec<(usize, String)> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let Event::Start(Tag::Heading { level, .. }) = event else {
            continue;
        };
        let text = heading_text(&events[index + 1..]);
        let anchor = unique_anchor(page.slug, &guide::anchor(&text), &mut used);
        entries.push(Entry { level: heading_level(*level), text, anchor: anchor.clone() });
        heading_ids.push((index, anchor));
    }

    for (index, anchor) in heading_ids {
        if let Event::Start(Tag::Heading { id, .. }) = &mut events[index] {
            *id = Some(CowStr::from(anchor));
        }
    }

    // Pass two: repoint every link. Intra-guide links become anchors in this document; links that
    // escape the guide tree become absolute GitHub URLs, since their targets are not in the zip.
    for event in &mut events {
        match event {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => {
                *dest_url = CowStr::from(rewrite_link(dest_url, page.slug, repo_ref));
            }
            _ => {}
        }
    }

    let mut rendered = String::with_capacity(page.markdown.len() * 2);
    html::push_html(&mut rendered, events.into_iter());
    (rendered, entries)
}

/// The plain text of a heading, read from the events that follow its opening tag.
fn heading_text(rest: &[Event]) -> String {
    let mut text = String::new();
    for event in rest {
        match event {
            Event::End(TagEnd::Heading(_)) => break,
            Event::Text(part) | Event::Code(part) => text.push_str(part),
            _ => {}
        }
    }
    text.trim().to_string()
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Namespace an anchor by its page and keep it unique.
///
/// Every page lands in the same document, so `## Flags` on three pages would otherwise collide and
/// send two thirds of those links to the wrong place. The `<page>--<anchor>` shape keeps the
/// original GitHub anchor visible, so a link copied out of the online docs is still recognisable.
fn unique_anchor(slug: &str, anchor: &str, used: &mut Vec<String>) -> String {
    let base = format!("{slug}--{anchor}");
    let mut candidate = base.clone();
    let mut counter = 1;
    while used.contains(&candidate) {
        candidate = format!("{base}-{counter}");
        counter += 1;
    }
    used.push(candidate.clone());
    candidate
}

/// Repoint one Markdown link target for the single-file document.
fn rewrite_link(dest: &str, page_slug: &str, repo_ref: &str) -> String {
    if dest.starts_with("http://") || dest.starts_with("https://") || dest.starts_with("mailto:") {
        return dest.to_string();
    }

    // A same-page anchor has to be namespaced like every other heading in this document.
    if let Some(fragment) = dest.strip_prefix('#') {
        return format!("#{page_slug}--{fragment}");
    }

    let (path, fragment) = match dest.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (dest, None),
    };

    // Anything climbing out of docs/guide/ is not in the zip; send it to GitHub.
    if path.starts_with("../") {
        let target = normalize(&format!("{GUIDE_DIR}/{path}"));
        // No filesystem here to ask, and the released binary would have nothing to ask anyway:
        // GitHub serves files under /blob/ and directories under /tree/, and an extension is a
        // reliable enough tell for the handful of outbound links the guide actually has.
        let route = if Path::new(&target).extension().is_some() { "blob" } else { "tree" };
        let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
        return format!("{REPO_WEB_BASE}/{route}/{repo_ref}/{target}{fragment}");
    }

    // A sibling guide page is a section of this document. The kind has to be checked: reference
    // pages are embedded in the binary too, so a bare `dataassets-internals.md` would otherwise
    // resolve to `#dataassets-internals` — an anchor for a section this document never renders.
    if let Some(page) = path.strip_suffix(".md").and_then(guide::page) {
        return match page.kind {
            Kind::Guide => match fragment {
                Some(fragment) => format!("#{}--{fragment}", page.slug),
                None => format!("#{}", page.slug),
            },
            // Not in the zip and not in this document, so it goes to GitHub like any other
            // outbound link.
            Kind::Reference => {
                // `Page::file` is already docs-relative, e.g. `reference/dataassets-internals.md`.
                let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
                format!("{REPO_WEB_BASE}/blob/{repo_ref}/docs/{}{fragment}", page.file)
            }
        };
    }

    dest.to_string()
}

/// Resolve `.` and `..` in a `/`-separated path without touching the filesystem.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

fn shell(nav: &str, body: &str) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GORE guide</title>
<style>{CSS}</style>
</head>
<body>
<nav id="sidebar">
  <div class="brand">GORE guide <span class="version">{version}</span></div>
  <input id="filter" type="search" placeholder="Filter pages and sections…" autocomplete="off" spellcheck="false">
  <ul id="nav">{nav}</ul>
</nav>
<main id="content">{body}</main>
<script>{JS}</script>
</body>
</html>
"##,
        version = escape(env!("CARGO_PKG_VERSION")),
    )
}

const CSS: &str = r#"
:root {
  --bg: #ffffff; --fg: #1f2328; --muted: #59636e; --line: #d1d9e0;
  --accent: #0969da; --code-bg: #f6f8fa; --side-bg: #f6f8fa; --mark: #fff3c4;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0d1117; --fg: #e6edf3; --muted: #9198a1; --line: #3d444d;
    --accent: #4493f8; --code-bg: #151b23; --side-bg: #010409; --mark: #4a3d00;
  }
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0; display: flex; align-items: flex-start;
  background: var(--bg); color: var(--fg);
  font: 16px/1.6 -apple-system, "Segoe UI", system-ui, sans-serif;
}
#sidebar {
  position: sticky; top: 0; flex: 0 0 19rem; width: 19rem; height: 100vh;
  overflow-y: auto; padding: 1rem; border-right: 1px solid var(--line);
  background: var(--side-bg);
}
.brand { font-weight: 600; font-size: 1.05rem; margin-bottom: .75rem; }
.version { color: var(--muted); font-weight: 400; font-size: .8rem; }
#filter {
  width: 100%; padding: .4rem .6rem; margin-bottom: .75rem;
  border: 1px solid var(--line); border-radius: 6px;
  background: var(--bg); color: var(--fg); font: inherit; font-size: .875rem;
}
#nav, #nav ul { list-style: none; margin: 0; padding: 0; }
#nav li.hidden { display: none; }

/* One page = one row = one hover target = one action. The triangle is drawn here rather than left
   to the native <summary> marker: the marker is a list-item bullet, and any block-level content
   beside it wraps onto its own line. */
summary, .nav-leaf {
  display: flex; align-items: center; gap: .45rem;
  padding: .25rem .4rem; border-radius: 4px;
  font-size: .875rem; font-weight: 600;
  color: var(--fg); text-decoration: none; cursor: pointer;
}
summary { list-style: none; }
summary::-webkit-details-marker { display: none; }
summary::before, .nav-leaf::before {
  content: ""; flex: none;
  border-left: 5px solid var(--muted);
  border-top: 4px solid transparent; border-bottom: 4px solid transparent;
  transition: transform .15s ease;
}
/* A page without subsections keeps the triangle's width so every label starts at one margin. */
.nav-leaf::before { border-left-color: transparent; }
details[open] > summary::before { transform: rotate(90deg); }
summary:hover, .nav-leaf:hover { background: var(--code-bg); }
summary.current, .nav-leaf.current { color: var(--accent); }

details > ul { margin: .1rem 0 .5rem 1.55rem; }
details > ul a {
  display: block; padding: .2rem .4rem; border-radius: 4px;
  color: var(--muted); font-size: .8125rem; text-decoration: none;
}
details > ul a:hover { background: var(--code-bg); }
details > ul a.current { color: var(--accent); font-weight: 600; }
main { flex: 1 1 auto; min-width: 0; padding: 2rem clamp(1rem, 4vw, 3.5rem) 6rem; }
.page { max-width: 52rem; }
.page + .page { margin-top: 3rem; padding-top: 2rem; border-top: 2px solid var(--line); }
h1, h2, h3, h4 { line-height: 1.3; margin: 1.6em 0 .6em; scroll-margin-top: 1rem; }
h1 { font-size: 1.9rem; margin-top: 0; }
h2 { font-size: 1.4rem; padding-bottom: .25em; border-bottom: 1px solid var(--line); }
h3 { font-size: 1.15rem; }
a { color: var(--accent); }
code {
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: .875em;
  background: var(--code-bg); padding: .15em .35em; border-radius: 4px;
}
pre {
  background: var(--code-bg); padding: .9rem 1rem; border-radius: 6px;
  overflow-x: auto; border: 1px solid var(--line);
}
pre code { background: none; padding: 0; font-size: .8125rem; line-height: 1.5; }
/* max-content + overflow keeps a wide table scrollable inside the column instead of
   forcing the whole page sideways. */
table {
  display: block; width: max-content; max-width: 100%; overflow-x: auto;
  border-collapse: collapse; margin: 1rem 0; font-size: .9rem;
}
th, td { border: 1px solid var(--line); padding: .4rem .7rem; text-align: left; vertical-align: top; }
th { background: var(--code-bg); }
blockquote {
  margin: 1rem 0; padding: .1rem 1rem; color: var(--muted);
  border-left: .25rem solid var(--line);
}
hr { border: 0; border-top: 1px solid var(--line); margin: 2rem 0; }
img { max-width: 100%; }
mark { background: var(--mark); color: inherit; }
@media (max-width: 60rem) {
  body { flex-direction: column; }
  #sidebar { position: static; width: 100%; flex-basis: auto; height: auto; max-height: 60vh; border-right: 0; border-bottom: 1px solid var(--line); }
  main { padding: 1.25rem; }
}
"#;

// `r##` rather than `r#`: the script contains the selector `a[href^="#"]`, whose `"#` would
// otherwise close the raw string.
const JS: &str = r##"
(function () {
  var nav = document.getElementById('nav');
  var filter = document.getElementById('filter');
  var pages = Array.prototype.slice.call(nav.querySelectorAll('li.nav-page'));

  function detailsOf(page) {
    var first = page.firstElementChild;
    return first && first.tagName === 'DETAILS' ? first : null;
  }

  // Expanding a page also goes to it. Bound to `click` rather than the `toggle` event on
  // purpose: `toggle` also fires when the scroll-spy below opens a page, which would turn
  // scrolling into navigation and fight the reader. A click is always the reader.
  Array.prototype.forEach.call(nav.querySelectorAll('summary'), function (summary) {
    summary.addEventListener('click', function () {
      // Runs before the open state flips, so `!open` means "about to expand".
      if (!summary.parentElement.open) {
        var target = summary.getAttribute('data-target');
        setTimeout(function () { location.hash = target; }, 0);
      }
    });
  });

  // Filter: hide entries that do not match, but keep a page visible while one of its sections
  // still matches, so a hit is never orphaned from its page. Matching pages are expanded for as
  // long as the filter is active and collapse again when it is cleared.
  filter.addEventListener('input', function () {
    var needle = filter.value.trim().toLowerCase();
    pages.forEach(function (page) {
      var details = detailsOf(page);
      var head = details ? details.querySelector('summary') : page.firstElementChild;
      var titleHit = head.textContent.toLowerCase().indexOf(needle) !== -1;

      var hits = 0;
      if (details) {
        Array.prototype.forEach.call(details.querySelectorAll('li'), function (item) {
          var hit = !needle || titleHit || item.textContent.toLowerCase().indexOf(needle) !== -1;
          item.classList.toggle('hidden', !hit);
          if (hit) hits++;
        });
      }

      var visible = !needle || titleHit || hits > 0;
      page.classList.toggle('hidden', !visible);
      if (details) details.open = !!needle && visible;
    });
  });

  // Scroll-spy over the page sections, so the sidebar always says where you are. A page is
  // represented by its summary, a section by its link, so both kinds of row can light up.
  var byHref = {};
  Array.prototype.forEach.call(nav.querySelectorAll('a[href^="#"]'), function (link) {
    byHref[link.getAttribute('href')] = link;
  });
  Array.prototype.forEach.call(nav.querySelectorAll('summary[data-target]'), function (summary) {
    byHref['#' + summary.getAttribute('data-target')] = summary;
  });
  var current = null;
  // Only ever collapse what we expanded ourselves -- a page the reader opened by hand stays open.
  var autoOpened = null;

  var observer = new IntersectionObserver(function (entries) {
    entries.forEach(function (entry) {
      if (!entry.isIntersecting) return;
      var link = byHref['#' + entry.target.id];
      if (!link || link === current) return;
      if (current) current.classList.remove('current');
      link.classList.add('current');
      current = link;

      var details = link.closest('details');
      if (details && details !== autoOpened) {
        if (autoOpened) autoOpened.open = false;
        details.open = true;
        autoOpened = details;
      }
    });
  }, { rootMargin: '0px 0px -80% 0px' });
  document.querySelectorAll('.page, h2[id]').forEach(function (node) { observer.observe(node); });
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> String {
        render("deadbeef")
    }

    #[test]
    fn every_embedded_page_becomes_a_section() {
        let html = document();
        for page in guide::guide_pages() {
            assert!(
                html.contains(&format!("id=\"{}\"", page.slug)),
                "{} is missing from the rendered document",
                page.slug
            );
        }
    }

    #[test]
    fn the_document_is_self_contained() {
        let html = document();
        // A page that reaches out to a CDN, a stylesheet or a neighbouring file is exactly the
        // failure this renderer exists to avoid: over file:// the browser blocks it.
        assert!(!html.contains("<link "), "no external stylesheet may be referenced");
        assert!(!html.contains("<script src"), "no external script may be referenced");
        assert!(!html.contains("fetch("), "nothing may be loaded at runtime");
        assert!(html.contains("<style>") && html.contains("<script>"));
    }

    #[test]
    fn reference_pages_stay_out_of_the_browsable_guide() {
        // gore.exe embeds docs/reference/ too, so the MCP server can explain a refusal. That
        // material is contracts, not instructions, and putting it in front of a reader is the
        // problem this split exists to fix.
        let html = document();
        for page in guide::pages_of(Kind::Reference) {
            assert!(
                !html.contains(&format!("id=\"{}\"", page.slug)),
                "{} is reference material and must not be rendered into the guide",
                page.slug
            );
        }
    }

    #[test]
    fn the_sidebar_is_one_collapsed_entry_per_page() {
        let html = document();
        assert_eq!(
            html.matches("class=\"nav-page").count(),
            guide::guide_pages().count(),
            "every page needs exactly one sidebar entry"
        );
        assert!(html.contains("<details><summary "), "pages with sections must be collapsible");
        assert!(
            !html.contains("<details open"),
            "nothing may start expanded — a collapsed sidebar is the whole point"
        );
        // A link nested inside a <summary> is what made the row have two click targets with two
        // outcomes under one hover. The summary carries a data-target instead.
        assert!(!html.contains("<summary><a"), "the page title must not be a nested link");
        assert!(html.contains("<summary data-target="));
    }

    #[test]
    fn tables_are_rendered_as_tables() {
        // The guide is table-heavy, and without ENABLE_TABLES they come out as literal pipes --
        // which is precisely the Notepad experience this command replaces.
        assert!(document().contains("<table>"));
    }

    #[test]
    fn every_internal_link_points_at_something_in_the_document() {
        let html = document();
        let mut ids: Vec<&str> = Vec::new();
        let mut rest = html.as_str();
        while let Some(at) = rest.find("id=\"") {
            rest = &rest[at + 4..];
            let end = rest.find('"').expect("unterminated id attribute");
            ids.push(&rest[..end]);
            rest = &rest[end..];
        }

        let mut rest = html.as_str();
        let mut checked = 0;
        while let Some(at) = rest.find("href=\"#") {
            rest = &rest[at + 7..];
            let end = rest.find('"').expect("unterminated href attribute");
            let target = &rest[..end];
            rest = &rest[end..];
            assert!(ids.contains(&target), "dangling in-document link #{target}");
            checked += 1;
        }
        assert!(checked > 100, "expected the guide's cross-links, found {checked}");

        // Sidebar page rows navigate through `data-target`, not `href`, so they need the same check.
        let mut rest = html.as_str();
        while let Some(at) = rest.find("data-target=\"") {
            rest = &rest[at + 13..];
            let end = rest.find('"').expect("unterminated data-target attribute");
            let target = &rest[..end];
            rest = &rest[end..];
            assert!(ids.contains(&target), "sidebar row points at a missing section #{target}");
        }
    }

    #[test]
    fn a_link_to_another_page_becomes_an_anchor() {
        assert_eq!(rewrite_link("textures.md", "items", "abc"), "#textures");
        assert_eq!(rewrite_link("textures.md#deploy", "items", "abc"), "#textures--deploy");
    }

    #[test]
    fn a_same_page_anchor_is_namespaced_by_its_page() {
        assert_eq!(rewrite_link("#other-helpers", "cli-reference", "abc"), "#cli-reference--other-helpers");
    }

    #[test]
    fn an_outbound_link_is_pinned_to_the_given_ref() {
        assert_eq!(
            rewrite_link("../../apps/save-editor/README.md", "items", "abc123"),
            "https://github.com/dh0er/gore/blob/abc123/apps/save-editor/README.md"
        );
        // No extension: GitHub serves directories under /tree/.
        assert_eq!(
            rewrite_link("../../crates/gore-tex", "building", "abc123"),
            "https://github.com/dh0er/gore/tree/abc123/crates/gore-tex"
        );
    }

    #[test]
    fn an_external_link_is_left_alone() {
        let url = "https://modelcontextprotocol.io";
        assert_eq!(rewrite_link(url, "mcp", "abc"), url);
    }

    #[test]
    fn a_sibling_link_to_a_reference_page_goes_to_github_not_a_dead_anchor() {
        // Reference pages are embedded in the binary but never rendered into this document, so
        // treating one as an in-document section would mint an anchor with nothing behind it.
        let reference = guide::pages_of(Kind::Reference).next().expect("a reference page");
        let sibling = format!("{}.md", reference.slug);

        let rewritten = rewrite_link(&sibling, "dataassets", "abc123");
        assert!(
            rewritten.starts_with("https://github.com/dh0er/gore/blob/abc123/docs/reference/"),
            "got {rewritten}"
        );
        assert!(!rewritten.starts_with('#'), "must not become an in-document anchor");

        // The `../reference/…` spelling the guide actually uses keeps working too.
        assert_eq!(
            rewrite_link("../reference/dataassets-internals.md", "dataassets", "abc123"),
            "https://github.com/dh0er/gore/blob/abc123/docs/reference/dataassets-internals.md"
        );
    }

    #[test]
    fn a_link_to_a_file_that_is_not_a_guide_page_is_left_alone() {
        assert_eq!(rewrite_link("overrides.toml", "items", "abc"), "overrides.toml");
        assert_eq!(rewrite_link("nope.md", "items", "abc"), "nope.md");
    }

    #[test]
    fn colliding_headings_across_pages_get_distinct_anchors() {
        // Two pages both have a `## Flags`; in one document those would otherwise be the same id.
        let mut used = Vec::new();
        assert_eq!(unique_anchor("audio", "flags", &mut used), "audio--flags");
        assert_eq!(unique_anchor("voice", "flags", &mut used), "voice--flags");
        assert_eq!(unique_anchor("audio", "flags", &mut used), "audio--flags-1");
    }

    #[test]
    fn paths_are_normalized_without_the_filesystem() {
        assert_eq!(normalize("docs/guide/../../lua/README.md"), "lua/README.md");
        assert_eq!(normalize("docs/guide/./x"), "docs/guide/x");
    }

    #[test]
    fn heading_text_stops_at_the_end_of_its_heading() {
        let events: Vec<Event> = Parser::new("# Title `code`\n\nbody text\n").collect();
        let start = events
            .iter()
            .position(|event| matches!(event, Event::Start(Tag::Heading { .. })))
            .unwrap();
        assert_eq!(heading_text(&events[start + 1..]), "Title code");
    }

    #[test]
    fn sidebar_titles_are_escaped() {
        assert_eq!(escape("a & b <c>"), "a &amp; b &lt;c&gt;");
    }

    #[test]
    fn a_search_listing_says_where_each_hit_is_and_why_it_won() {
        // This subcommand exists to make the ranking inspectable from a shell, so the two things a
        // listing must carry are the score that decided the order and the file the hit is in.
        let query = "replace a texture";
        let hits = guide::search::search(query, 3);
        let listing = search_listing(query, &hits);

        assert!(listing.starts_with("3 section(s) match"), "{listing}");
        assert!(listing.contains("docs/guide/textures.md"), "{listing}");
        assert!(listing.contains(&format!("[{}]", hits[0].score)), "{listing}");
        assert!(listing.contains(&format!("{}#{}", hits[0].page, hits[0].anchor)), "{listing}");
    }

    #[test]
    fn a_search_that_matches_nothing_says_what_to_try_instead() {
        // Rather than printing an empty list, which reads as a broken command, or failing, which
        // this is not: the guide genuinely does not cover everything.
        let listing = search_listing("zzzznotawordanywhere", &[]);
        assert!(listing.contains("nothing in the guide matches"), "{listing}");
        assert!(listing.contains("gore guide html"), "{listing}");
    }

    #[test]
    fn a_reference_hit_is_labelled_as_reference_and_not_as_guide() {
        // The two bodies answer different questions, and a shell user picking a hit off this list
        // is entitled to know which one they are about to open: the guide says which command to
        // reach for, the reference says why one refused.
        //
        // The query has to be one no single page owns outright. A page slug is scored for
        // every section on that page, so once a guide page is called `npc-authoring`,
        // "npc authoring" returns eight sections of it and nothing else — the ranking
        // working, not a labelling regression. "dialog runtime" spans both bodies as that
        // one used to.
        let query = "dialog runtime";
        let hits = guide::search::search(query, 8);
        let listing = search_listing(query, &hits);
        assert!(listing.contains("reference · docs/reference/"), "{listing}");
    }

    #[test]
    fn several_words_are_one_query_rather_than_several() {
        // `gore guide search click sound music menu` has to mean what the MCP tool means by the
        // same string, or the shell form is debugging something else.
        let terms: Vec<String> =
            ["click", "sound", "music", "menu"].iter().map(|word| word.to_string()).collect();
        assert_eq!(
            guide::search::search(&terms.join(" "), 5),
            guide::search::search("click sound music menu", 5)
        );
    }
}
