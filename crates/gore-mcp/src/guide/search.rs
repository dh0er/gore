//! Full-text search over the embedded guide.
//!
//! Deliberately simple and dependency-free: term counting with a few positional weights, then a
//! bonus when a single section matches every term. There is no stemming, no index, and no fuzzy
//! matching. Twenty-one pages of technical prose is small enough that scanning them is instant, and
//! the queries an agent asks ("texture replace", "how do I splice a module") are keyword queries
//! that this handles well.
//!
//! Ranking sections rather than pages is the important part. `dataassets.md` is 747 lines; telling
//! an agent "the answer is somewhere in there" would push it to read the whole page. Pointing at
//! one heading keeps the follow-up read small.

use super::{Section, PAGES};

/// Beyond this, extra terms are noise rather than signal.
const MAX_TERMS: usize = 8;

/// Characters of context on either side of the first match in the snippet.
const SNIPPET_RADIUS: usize = 120;

/// A term appearing in the page slug is a strong signal: the slug *is* the topic.
const WEIGHT_SLUG: u32 = 12;
const WEIGHT_TOP_HEADING: u32 = 8;
const WEIGHT_SUB_HEADING: u32 = 4;
/// Body occurrences count, but a long section should not win on length alone.
const WEIGHT_BODY: u32 = 1;
const MAX_BODY_HITS: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub page: &'static str,
    pub anchor: String,
    pub heading: String,
    pub score: u32,
    pub snippet: String,
}

/// Rank guide sections against a query. Returns at most `limit` hits, best first.
pub fn search(query: &str, limit: usize) -> Vec<Hit> {
    let terms: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .take(MAX_TERMS)
        .map(str::to_string)
        .collect();

    if terms.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<Hit> = Vec::new();
    for page in PAGES {
        let slug = page.slug.to_lowercase();
        for section in page.sections() {
            if let Some(hit) = score_section(page.slug, &slug, &section, &terms) {
                hits.push(hit);
            }
        }
    }

    // Ties break on page then anchor so repeated identical queries return identical output — a
    // property worth having when a model compares two answers.
    hits.sort_by(|a, b| {
        b.score.cmp(&a.score).then_with(|| a.page.cmp(b.page)).then_with(|| a.anchor.cmp(&b.anchor))
    });
    hits.truncate(limit);
    hits
}

fn score_section(
    page: &'static str,
    slug_lower: &str,
    section: &Section<'_>,
    terms: &[String],
) -> Option<Hit> {
    let heading_lower = section.heading.to_lowercase();
    let body_lower = section.body.to_lowercase();

    let mut score = 0u32;
    let mut matched_all = true;
    let mut first_match: Option<usize> = None;

    for term in terms {
        let mut term_score = 0u32;

        if slug_lower.contains(term.as_str()) {
            term_score += WEIGHT_SLUG;
        }
        if heading_lower.contains(term.as_str()) {
            term_score +=
                if section.level <= 2 { WEIGHT_TOP_HEADING } else { WEIGHT_SUB_HEADING };
        }

        let occurrences = body_lower.matches(term.as_str()).count() as u32;
        term_score += WEIGHT_BODY * occurrences.min(MAX_BODY_HITS);

        if let Some(position) = body_lower.find(term.as_str()) {
            first_match = Some(first_match.map_or(position, |current| current.min(position)));
        }

        if term_score == 0 {
            matched_all = false;
        }
        score += term_score;
    }

    if score == 0 {
        return None;
    }
    // A section that covers the whole query beats one that happens to mention a single word of it.
    if matched_all && terms.len() > 1 {
        score *= 2;
    }

    Some(Hit {
        page,
        anchor: section.anchor.clone(),
        heading: section.heading.to_string(),
        score,
        snippet: snippet(section.body, first_match.unwrap_or(0)),
    })
}

/// A readable window around `position`, clamped to character boundaries.
fn snippet(body: &str, position: usize) -> String {
    let start = floor_boundary(body, position.saturating_sub(SNIPPET_RADIUS));
    let end = ceil_boundary(body, (position + SNIPPET_RADIUS).min(body.len()));

    let mut snippet = String::new();
    if start > 0 {
        snippet.push('…');
    }
    snippet.push_str(body[start..end].trim());
    if end < body.len() {
        snippet.push('…');
    }
    // Collapse to one line: a snippet is a pointer, not an excerpt.
    snippet.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn floor_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_returns_nothing_rather_than_everything() {
        assert!(search("", 10).is_empty());
        assert!(search("   ", 10).is_empty());
    }

    #[test]
    fn a_topic_word_ranks_its_own_page_first() {
        let hits = search("texture", 5);
        assert_eq!(hits[0].page, "textures", "got {:?}", hits.iter().map(|h| h.page).collect::<Vec<_>>());
    }

    #[test]
    fn a_multi_word_query_prefers_a_section_covering_all_of_it() {
        let hits = search("splice module cache", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].page, "scripts", "got {:?}", hits.iter().map(|h| h.page).collect::<Vec<_>>());
    }

    #[test]
    fn results_are_capped_and_ordered_by_score() {
        let hits = search("mod", 4);
        assert!(hits.len() <= 4);
        for pair in hits.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
    }

    #[test]
    fn the_same_query_always_returns_the_same_order() {
        assert_eq!(search("deploy bundle", 10), search("deploy bundle", 10));
    }

    #[test]
    fn a_query_matching_nothing_returns_nothing() {
        assert!(search("zzzznotawordanywhere", 10).is_empty());
    }

    #[test]
    fn every_hit_carries_a_usable_pointer_back_into_the_guide() {
        for hit in search("localization", 5) {
            let page = super::super::page(hit.page).expect("hit names a real page");
            assert!(
                page.section(&hit.anchor).is_some(),
                "{}#{} does not resolve",
                hit.page,
                hit.anchor
            );
            assert!(!hit.snippet.is_empty());
        }
    }

    #[test]
    fn snippets_stay_on_character_boundaries() {
        // A multi-byte character sitting exactly on the window edge must not panic.
        let body = "ä".repeat(400);
        let snippet = snippet(&body, 200);
        assert!(!snippet.is_empty());
    }

    #[test]
    fn a_snippet_is_a_single_line() {
        for hit in search("deploy", 10) {
            assert!(!hit.snippet.contains('\n'), "{:?}", hit.snippet);
        }
    }
}
