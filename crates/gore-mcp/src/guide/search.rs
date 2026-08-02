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
//!
//! Two corrections to that simplicity, both from watching a real session spend nine searches and
//! then read six whole pages anyway:
//!
//! - A term matches where a word begins, never inside one. Plain `contains` made `ui` a hit on
//!   `b`**`ui`**`ld` and `g`**`ui`**`de`, which is how a query about a UI sound came back holding
//!   the `guide` CLI page.
//! - A term is worth less the more of the corpus it appears in. Every modding query contains the
//!   word "game", and without damping that word alone was worth 25 points before anything specific
//!   scored at all.

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

/// One section that matched, before the corpus-wide weights are known.
struct Candidate {
    page: &'static str,
    anchor: String,
    heading: String,
    body: &'static str,
    /// Where the snippet should start.
    first_match: usize,
    /// One raw score per query term, positionally.
    per_term: Vec<u32>,
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

    // Scoring is two passes over one collection of raw numbers rather than two passes over the
    // pages: how much a term is worth depends on how many sections hold it, which is not known
    // until every section has been looked at.
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut sections_holding: Vec<usize> = vec![0; terms.len()];
    let mut total_sections = 0usize;

    for page in PAGES {
        let slug = page.slug.to_lowercase();
        for section in page.sections() {
            total_sections += 1;
            if let Some(candidate) = score_section(page.slug, &slug, &section, &terms) {
                for (index, raw) in candidate.per_term.iter().enumerate() {
                    if *raw > 0 {
                        sections_holding[index] += 1;
                    }
                }
                candidates.push(candidate);
            }
        }
    }

    let weights: Vec<f64> =
        sections_holding.iter().map(|holding| rarity(total_sections, *holding)).collect();

    let mut hits: Vec<Hit> = candidates
        .into_iter()
        .map(|candidate| {
            let mut score = 0.0f64;
            let mut matched_all = true;
            for (index, raw) in candidate.per_term.iter().enumerate() {
                if *raw == 0 {
                    matched_all = false;
                }
                score += f64::from(*raw) * weights[index];
            }
            // A section that covers the whole query beats one that happens to mention a single
            // word of it.
            if matched_all && terms.len() > 1 {
                score *= 2.0;
            }
            Hit {
                page: candidate.page,
                anchor: candidate.anchor,
                heading: candidate.heading,
                score: score.round() as u32,
                snippet: snippet(candidate.body, candidate.first_match),
            }
        })
        .collect();

    // Ties break on page then anchor so repeated identical queries return identical output — a
    // property worth having when a model compares two answers.
    hits.sort_by(|a, b| {
        b.score.cmp(&a.score).then_with(|| a.page.cmp(b.page)).then_with(|| a.anchor.cmp(&b.anchor))
    });
    hits.truncate(limit);
    hits
}

/// What one term is worth, given how much of the corpus contains it.
///
/// `1 + ln(sections / sections holding the term)`: a term in a handful of sections counts several
/// times over one that is everywhere, and a term in every single section still counts once rather
/// than nothing — a query made only of common words must return its best sections, not an empty
/// list.
///
/// Measured rather than listed. A stoplist would have to name the words this particular corpus is
/// saturated with (`game`, `mod`, `file`), and every page added afterwards would make that list a
/// little less true with nothing to say so. This is derived from the pages themselves, so it is
/// correct by construction and costs one pass.
fn rarity(total_sections: usize, sections_holding: usize) -> f64 {
    if total_sections == 0 || sections_holding == 0 {
        return 1.0;
    }
    1.0 + (total_sections as f64 / sections_holding as f64).ln()
}

fn score_section(
    page: &'static str,
    slug_lower: &str,
    section: &Section<'static>,
    terms: &[String],
) -> Option<Candidate> {
    let heading_lower = section.heading.to_lowercase();
    let body_lower = section.body.to_lowercase();

    let mut per_term = Vec::with_capacity(terms.len());
    let mut any = false;
    let mut first_match: Option<usize> = None;

    for term in terms {
        let mut term_score = 0u32;

        if token_hits(slug_lower, term) > 0 {
            term_score += WEIGHT_SLUG;
        }
        if token_hits(&heading_lower, term) > 0 {
            term_score +=
                if section.level <= 2 { WEIGHT_TOP_HEADING } else { WEIGHT_SUB_HEADING };
        }

        let occurrences = token_hits(&body_lower, term);
        term_score += WEIGHT_BODY * occurrences.min(MAX_BODY_HITS);

        if let Some(position) = token_position(&body_lower, term) {
            first_match = Some(first_match.map_or(position, |current| current.min(position)));
        }

        any |= term_score > 0;
        per_term.push(term_score);
    }

    any.then(|| Candidate {
        page,
        anchor: section.anchor.clone(),
        heading: section.heading.to_string(),
        body: section.body,
        first_match: first_match.unwrap_or(0),
        per_term,
    })
}

/// How often `term` starts a word in `haystack`.
///
/// A match must begin at a word boundary; it need not end at one, so `texture` still finds
/// `textures` and `bank` still finds `banks`, which is how the guide is actually written. What it
/// no longer finds is `ui` inside `build`, `guide` and `quick` — three words that between them
/// appear on every page of the guide, and that made a query about a UI sound return the CLI
/// reference.
fn token_hits(haystack: &str, term: &str) -> u32 {
    let mut hits = 0;
    let mut from = 0usize;
    while let Some(offset) = haystack[from..].find(term) {
        let at = from + offset;
        if starts_a_word(haystack, at) {
            hits += 1;
        }
        // `term` matched here, so this is a character boundary.
        from = at + term.len();
    }
    hits
}

/// The first word-boundary occurrence, for the snippet window.
fn token_position(haystack: &str, term: &str) -> Option<usize> {
    let mut from = 0usize;
    while let Some(offset) = haystack[from..].find(term) {
        let at = from + offset;
        if starts_a_word(haystack, at) {
            return Some(at);
        }
        from = at + term.len();
    }
    None
}

fn starts_a_word(haystack: &str, at: usize) -> bool {
    !haystack[..at].chars().next_back().is_some_and(char::is_alphanumeric)
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
    fn a_term_matches_where_a_word_starts_and_never_inside_one() {
        // `contains` is what made `ui` a hit on `build` and `guide`. Both words are on every page
        // of the guide, so the two-letter query scored the whole corpus and ranked it by length.
        assert_eq!(token_hits("build", "ui"), 0);
        assert_eq!(token_hits("guide", "ui"), 0);
        assert_eq!(token_hits("the guide builds quickly", "ui"), 0);
        // Underscores and hyphens separate words, which is how the sample names an agent searches
        // for are spelled: `SFX_UI_Action_Button_Click_01`.
        assert_eq!(token_hits("sfx_ui_action_button_click_01", "ui"), 1);
        assert_eq!(token_hits("ui", "ui"), 1);
        // The end of the term is deliberately not a boundary: the guide writes plurals, and a
        // query does not.
        assert_eq!(token_hits("textures", "texture"), 1);
        assert_eq!(token_hits("one bank, two banks", "bank"), 2);
    }

    #[test]
    fn a_query_about_a_ui_sound_ranks_the_audio_page_first() {
        // The query a real session ran, verbatim. It used to return the `guide` CLI page and two
        // studio-internals sections, because `ui` matched `b-ui-ld` and `g-ui-de`, and the session
        // recovered by reading six whole pages. Dropping three words from it used to be the only
        // way to reach `audio`.
        let hits = search("UI click sound button music main menu", 5);
        assert_eq!(
            hits[0].page,
            "audio",
            "got {:?}",
            hits.iter().map(|hit| format!("{}#{}", hit.page, hit.anchor)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_word_the_whole_corpus_uses_does_not_decide_the_ranking() {
        // `game-updates` took the top slot on all three of these in one session: 12 for the slug,
        // 8 for the heading and 5 body hits is 25 points from the word "game" alone, and every
        // modding query contains it.
        for query in [
            "spawn npc start location new game",
            "player start location prison beach",
            "spawn existing npc teleport waypoint angelscript at game start",
        ] {
            let hits = search(query, 3);
            assert!(
                hits.iter().all(|hit| hit.page != "game-updates"),
                "{query:?} still ranks game-updates in the top three: {:?}",
                hits.iter().map(|hit| format!("{}#{}", hit.page, hit.anchor)).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_query_made_only_of_common_words_still_answers() {
        // The damping has a floor for this reason: a term in every section is worth little, never
        // nothing. Scoring it at zero would turn a vague query into "nothing matches", which reads
        // as "the guide does not cover this".
        assert!(!search("game", 5).is_empty());
        assert!(rarity(400, 400) >= 1.0);
        assert!(rarity(400, 4) > rarity(400, 200));
        assert!(rarity(400, 200) > rarity(400, 400));
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
