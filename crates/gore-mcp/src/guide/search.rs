//! Full-text search over the embedded guide.
//!
//! Deliberately simple and dependency-free: term counting with a few positional weights, a reward
//! for covering more of the query, and a preference for the guide over the reference. There is no
//! stemming, no index, and no fuzzy matching. Two dozen pages of technical prose is small enough
//! that scanning all of it on every query is instant.
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
//!
//! Those two assumed the caller writes keyword queries. A blind tester typed his problem the way he
//! would say it out loud — "deployed but nothing changed in game, mod has no effect" — and the top
//! hits were "Mod Studio", "Mod Studio voice authoring internals", "Cooked DataAsset internals" and
//! "Mod Studio project snapshot internals". He got nowhere until he read the table of contents by
//! hand. The guide had the answer and had titled it honestly; five things stood between the two.
//!
//! - A query is split into words, not into whitespace runs. `game,` used to be the literal term, so
//!   it only matched the comma-suffixed occurrences. See [`terms_of`].
//! - The cap on those words is large enough to hold a sentence. At eight whitespace chunks his
//!   query reached the scorer as `deployed but nothing changed in game, mod has` — `no effect`, the
//!   only part that named the symptom, was cut off, and three of the eight slots held `but`, `in`
//!   and `has`. See [`MAX_TERMS`].
//! - A section is scored on its *own* prose. Sections nest, so `# Textures` contains the whole page:
//!   seven of his eight hits were level-1 page titles, because a title section collects
//!   [`MAX_BODY_HITS`] for term after term and no real section can outbid that. Ranking sections is
//!   the entire point of this module, and it was quietly ranking pages. See [`score_section`].
//! - Covering more of the query beats mentioning one word of it many times, and a word is worth
//!   sharply more the rarer it is, so that the content words of a sentence outweigh its function
//!   words. See [`coverage_bonus`] and [`term_weight`].
//! - The guide outranks the reference unless the query's own vocabulary says the caller is asking
//!   about internals. The tool description already promised this — the reference is for "when a
//!   command refuses something and the guide does not say why" — and nothing in the ranking
//!   reflected it. See [`reference_lean`] and [`reference_share_of_prose`].

use super::{Kind, Page, Section, PAGES};

/// Beyond this, extra terms are noise rather than signal.
///
/// Sized for a sentence, not for keywords. A symptom typed in someone's own words runs ten to
/// fifteen words, and the words that name the symptom tend to come last — cutting at eight dropped
/// `no effect` from the query this number was raised for. The extra terms cost one more comparison
/// per section across a corpus of a few hundred, which is nothing, and [`term_weight`] prices `but`
/// and `in` at approximately zero.
const MAX_TERMS: usize = 16;

/// Characters of context on either side of the first match in the snippet.
const SNIPPET_RADIUS: usize = 120;

/// A term appearing in the page slug is a strong signal: the slug *is* the topic.
const WEIGHT_SLUG: u32 = 12;
const WEIGHT_TOP_HEADING: u32 = 12;
const WEIGHT_SUB_HEADING: u32 = 9;
/// Body occurrences count, but a long section should not win on length alone.
///
/// A heading is an editorial claim that the section is *about* the word; a body occurrence is only
/// evidence the word came up. The gap between the two is deliberately wide enough that no amount of
/// repetition in prose overtakes a heading match — "the section titles were good and the search
/// simply did not find them" is the report this answers.
const WEIGHT_BODY: u32 = 1;
const MAX_BODY_HITS: u32 = 3;

/// What a reference section is worth when the query does not read like a question about internals.
///
/// A demotion, never a filter. The reference pages are the right answer to some questions and stay
/// reachable for all of them: this only settles who goes first when a beginner's phrasing happens to
/// score the same against a receipt-semantics page as against the page that answers him.
const REFERENCE_OFF_TOPIC: f64 = 0.5;

/// How far past neutral a query's vocabulary must lean before it counts as a question about
/// internals: a tenth of the way from what an ordinary word leans to what a reference-only word
/// leans.
///
/// A margin rather than a bare comparison, because the thing being compared moves. Neutral is
/// [`reference_share_of_prose`], which shifts whenever a page is edited, and the first version of
/// this demanded only "above neutral". A page grew by a few paragraphs mid-development and the
/// tester's query crossed the line — 0.415 against a neutral point of 0.413 — putting a whole body
/// of maintainer internals back at the front of his results, for an edit to a page he was never
/// going to read.
///
/// A tenth is where the two kinds of query stop being close. Measured over the sets in
/// `symptom_queries_and_internals_queries_fall_on_opposite_sides_of_the_line`, symptom phrasings
/// ran 0.12–0.34 and internals phrasings 0.47–0.71, so the threshold sits in the gap rather than in
/// the crowd. That test re-measures both sets on every run and is the canary if the corpus drifts
/// far enough to close it.
const INTERNALS_MARGIN: f64 = 0.1;

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
    kind: Kind,
    anchor: String,
    heading: String,
    body: &'static str,
    /// Where the snippet should start.
    first_match: usize,
    /// One raw score per query term, positionally.
    per_term: Vec<u32>,
}

/// Split a query into the words that will be looked for.
///
/// Words, not whitespace runs. `split_whitespace` left the punctuation attached, so a query ending
/// `…in game, mod has no effect` searched for the literal term `game,` and matched only the
/// occurrences that happen to carry a comma. Anything that is not alphanumeric ends a word, except
/// `_`, which the guide uses inside the names an agent searches for (`SFX_UI_Action_…`).
///
/// A hyphen ends a word too, though the guide uses it in names like `mod-studio`. It has to: this
/// search takes prose, and prose gets hyphenated — `game-path configuration` kept `game-path` as
/// one term and could not score a section saying "game path", which is how the guide spells it.
/// Nothing is lost the other way, because a term matches at any word boundary and `-` is one:
/// `mod` and `studio` both hit the text `mod-studio`, and both hit its slug, so the section that
/// is actually about it scores twice rather than not at all.
///
/// Single characters are dropped: `item's damage` splits into `item`, `s`, `damage`, and `s` starts
/// a word in `so`, `sealed` and several thousand other places. Duplicates are dropped too, so a
/// repeated word cannot buy a section a second helping of the same evidence.
fn terms_of(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for word in query
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '_')
    {
        let word = word.trim_matches(|character| character == '_');
        if word.chars().count() < 2 || terms.iter().any(|seen| seen == word) {
            continue;
        }
        terms.push(word.to_string());
        if terms.len() == MAX_TERMS {
            break;
        }
    }
    terms
}

/// Rank guide sections against a query. Returns at most `limit` hits, best first.
pub fn search(query: &str, limit: usize) -> Vec<Hit> {
    let terms = terms_of(query);

    if terms.is_empty() {
        return Vec::new();
    }

    // Scoring is two passes over one collection of raw numbers rather than two passes over the
    // pages: how much a term is worth depends on how many sections hold it, which is not known
    // until every section has been looked at.
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut sections_holding: Vec<usize> = vec![0; terms.len()];
    let mut reference_holding: Vec<usize> = vec![0; terms.len()];
    let mut total_sections = 0usize;

    for page in PAGES {
        let slug = page.slug.to_lowercase();
        let sections = page.sections();
        for index in 0..sections.len() {
            total_sections += 1;
            if let Some(candidate) = score_section(page, &slug, &sections, index, &terms) {
                for (term, raw) in candidate.per_term.iter().enumerate() {
                    if *raw > 0 {
                        sections_holding[term] += 1;
                        if page.kind == Kind::Reference {
                            reference_holding[term] += 1;
                        }
                    }
                }
                candidates.push(candidate);
            }
        }
    }

    let weights: Vec<f64> =
        sections_holding.iter().map(|holding| term_weight(total_sections, *holding)).collect();
    let total_weight: f64 = weights.iter().sum();

    // Whether the reference body is what this query is reaching for, decided from the query's own
    // vocabulary rather than from a list of words someone thought sounded technical.
    let internals_query =
        reference_lean(&sections_holding, &reference_holding, &weights) > internals_threshold();

    let mut hits: Vec<Hit> = candidates
        .into_iter()
        .map(|candidate| {
            let mut score = 0.0f64;
            let mut covered = 0.0f64;
            for (index, raw) in candidate.per_term.iter().enumerate() {
                if *raw > 0 {
                    covered += weights[index];
                }
                score += f64::from(*raw) * weights[index];
            }
            score *= coverage_bonus(covered, total_weight);
            if candidate.kind == Kind::Reference && !internals_query {
                score *= REFERENCE_OFF_TOPIC;
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

/// What one term contributes, per point of raw score.
///
/// [`rarity`] squared. A keyword query is all content words and the difference hardly shows, but a
/// query typed as a sentence is mostly function words: "deployed but nothing changed in game, mod
/// has no effect" is ten terms of which four carry the symptom. Priced linearly, `but`, `in`, `has`,
/// `no` and `the` between them out-scored `deployed` and `effect`, and the top hit was a section of
/// the MCP page whose only claim on the query was the word "Effect" in a table header and enough
/// length to mention each function word once.
///
/// Squaring is what a corpus this small has instead of the length normalisation a real search engine
/// would apply: a long section can still collect every common word, but doing so is now worth almost
/// nothing next to one occurrence of the word that names the problem.
fn term_weight(total_sections: usize, sections_holding: usize) -> f64 {
    let rarity = rarity(total_sections, sections_holding);
    rarity * rarity
}

/// How much a section is rewarded for answering more of the query.
///
/// `1 + the share of the query's weight this section matched at all`, so covering everything is
/// worth exactly twice covering nothing — the multiplier the old all-or-nothing rule applied. The
/// difference is that this one is continuous, and that is what a sentence needs. "Deployed but
/// nothing changed in game, mod has no effect" is ten terms and the most any one section holds is
/// nine, so the old bonus never fired for it at all: the ranking fell back to "which section
/// repeats one of these words most", which is a question about length. Weighting the share by
/// [`term_weight`] is what keeps `deployed` and `effect` worth more of the bonus than `game` and
/// `has`.
fn coverage_bonus(covered_weight: f64, total_weight: f64) -> f64 {
    if total_weight <= 0.0 {
        return 1.0;
    }
    1.0 + (covered_weight / total_weight).clamp(0.0, 1.0)
}

/// How far the query's vocabulary leans into the reference body, from 0 (only ever seen in the
/// guide) to 1 (only ever seen in the reference).
///
/// Compared against [`internals_threshold`], this decides whether a search is asking about
/// internals. Measured rather than listed, for the same reason [`rarity`] is: a list of "internals
/// words" would be someone's guess at what the reference happens to be about today, and would rot
/// the first time a page was added. `receipt`, `seal`, `usmap` and `invariant` live almost entirely
/// in `docs/reference/`, so a query made of them leans high without anyone saying so; `deployed`,
/// `nothing` and `changed` are guide vocabulary and lean low.
///
/// Terms nothing holds are skipped — an unmatched word says nothing about which body is wanted.
fn reference_lean(sections_holding: &[usize], reference_holding: &[usize], weights: &[f64]) -> f64 {
    let mut leaning = 0.0f64;
    let mut counted = 0.0f64;
    for (index, holding) in sections_holding.iter().enumerate() {
        if *holding == 0 {
            continue;
        }
        leaning += weights[index] * (reference_holding[index] as f64 / *holding as f64);
        counted += weights[index];
    }
    if counted <= 0.0 {
        return 0.0;
    }
    leaning / counted
}

/// The share of the corpus's prose that is reference — the lean an ordinary word already has.
///
/// The neutral point [`reference_lean`] is measured from, and the obvious candidate — the
/// reference's share of the *section count* — is wrong in a way that matters. Reference sections are
/// longer, so an everyday word turns up in one more often than counting sections predicts: when
/// this was written `the` sat in 83 of the 248 sections holding it and `or` in 60 of 147, against a
/// section share of 0.32. Judged against that share, a query written in plain English leans "high"
/// for no reason but its function words, and "deployed but nothing changed in game, mod has no
/// effect" measured 0.400 — comfortably "internals", which is how four pages of receipt semantics
/// came back to a beginner.
///
/// This proxy is not assumed to be the right neutral point, it is checked against the real one:
/// `the_neutral_point_is_what_an_average_word_actually_leans` tokenises all 5 000-odd distinct words
/// in the corpus, computes what the average one leans, and fails if the two drift apart.
///
/// Note that correcting the neutral point is necessary and not sufficient. It moves that query from
/// well above neutral to roughly level with it — 0.410 against 0.405 at the time of writing — and
/// what actually settles the question is [`INTERNALS_MARGIN`].
fn reference_share_of_prose() -> f64 {
    let mut total = 0usize;
    let mut reference = 0usize;
    for page in PAGES {
        total += page.markdown.len();
        if page.kind == Kind::Reference {
            reference += page.markdown.len();
        }
    }
    if total == 0 {
        return 0.0;
    }
    reference as f64 / total as f64
}

/// The lean a query must beat to be treated as a question about internals.
fn internals_threshold() -> f64 {
    let neutral = reference_share_of_prose();
    neutral + INTERNALS_MARGIN * (1.0 - neutral)
}

/// Score one section, identified by its index in its page's sections in document order.
///
/// The index is what lets the body be narrowed to the section's *own* prose: sections nest, so
/// `section.body` for `# Textures` is the entire page and every `##` under it, and scoring that made
/// every page title outrank every real section on it. A section's own text runs from its heading to
/// the next heading of any level, which is exactly the prose its heading is a promise about.
fn score_section(
    page: &'static Page,
    slug_lower: &str,
    sections: &[Section<'static>],
    index: usize,
    terms: &[String],
) -> Option<Candidate> {
    let section = &sections[index];
    let own_end = sections.get(index + 1).map_or(page.markdown.len(), |next| next.start);
    let own = &page.markdown[section.start..own_end];

    let heading_lower = section.heading.to_lowercase();
    let own_lower = own.to_lowercase();

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

        let occurrences = token_hits(&own_lower, term);
        term_score += WEIGHT_BODY * occurrences.min(MAX_BODY_HITS);

        if let Some(position) = token_position(&own_lower, term) {
            first_match = Some(first_match.map_or(position, |current| current.min(position)));
        }

        any |= term_score > 0;
        per_term.push(term_score);
    }

    any.then(|| Candidate {
        page: page.slug,
        kind: page.kind,
        anchor: section.anchor.clone(),
        heading: section.heading.to_string(),
        body: own,
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

    /// `page#anchor` for every hit, which is what a failure message has to show to be worth reading.
    fn where_(hits: &[Hit]) -> Vec<String> {
        hits.iter().map(|hit| format!("{}#{}", hit.page, hit.anchor)).collect()
    }

    /// How far one query leans into the reference. The corpus pass is repeated here rather than
    /// factored out of [`search`], which needs it interleaved with collecting candidates; the
    /// policy — [`score_section`], [`term_weight`], [`reference_lean`] — is the same code the
    /// ranking runs.
    fn lean_of(query: &str) -> f64 {
        let terms = terms_of(query);
        let mut holding = vec![0usize; terms.len()];
        let mut reference_holding = vec![0usize; terms.len()];
        let mut total = 0usize;

        for page in PAGES {
            let slug = page.slug.to_lowercase();
            let sections = page.sections();
            for index in 0..sections.len() {
                total += 1;
                if let Some(candidate) = score_section(page, &slug, &sections, index, &terms) {
                    for (term, raw) in candidate.per_term.iter().enumerate() {
                        if *raw > 0 {
                            holding[term] += 1;
                            if page.kind == Kind::Reference {
                                reference_holding[term] += 1;
                            }
                        }
                    }
                }
            }
        }

        let weights: Vec<f64> = holding.iter().map(|held| term_weight(total, *held)).collect();
        reference_lean(&holding, &reference_holding, &weights)
    }

    #[test]
    fn symptom_queries_and_internals_queries_fall_on_opposite_sides_of_the_line() {
        // The canary for everything the guide-over-reference preference does. When a ranking test
        // further down starts failing, this says whether the classifier moved or the corpus did.
        //
        // The threshold is a margin past neutral rather than neutral itself for a reason this test
        // caught: on an earlier corpus the first query below measured 0.400 against a neutral point
        // of 0.422, a reference page grew by a few paragraphs mid-development, and it crossed to
        // 0.415 against 0.413 — a beginner's phrasing reclassified as a question about internals by
        // an edit to a page he was never going to read.
        let threshold = internals_threshold();

        for query in [
            "deployed but nothing changed in game, mod has no effect",
            "my mod does nothing after deploying",
            "changed a texture but the game looks the same",
            "I replaced the voice line but the character still says the old one",
            "I edited the german text but the dialog still shows the old line",
            "the game crashes when I start it with my mod",
            "how do I change an item's damage value",
        ] {
            let lean = lean_of(query);
            assert!(
                lean < threshold,
                "{query:?} leans {lean:.3} against a threshold of {threshold:.3}, so it would be \
                 answered out of the reference"
            );
        }

        for query in [
            "receipt seal mismatch invariant usmap generation",
            "usmap generation",
            "what does a receipt actually guarantee",
        ] {
            let lean = lean_of(query);
            assert!(
                lean > threshold,
                "{query:?} leans {lean:.3} against a threshold of {threshold:.3}, so the reference \
                 would be demoted for a question only the reference answers"
            );
        }
    }

    #[test]
    fn a_query_is_split_into_words_and_not_into_whitespace_runs() {
        // `split_whitespace` kept the punctuation, so the tester's query searched for the literal
        // term `game,` and matched only the occurrences carrying a comma.
        assert_eq!(terms_of("in game, mod has"), vec!["in", "game", "mod", "has"]);
        assert_eq!(terms_of("nothing changed."), vec!["nothing", "changed"]);
        // A one-character fragment is noise: `s` starts a word in `so`, `sealed` and thousands more.
        assert_eq!(terms_of("an item's damage"), vec!["an", "item", "damage"]);
        // An underscore is part of the names an agent searches for; a hyphen is not, because prose
        // gets hyphenated. `game-path configuration` kept `game-path` as one term and could not
        // score a section saying "game path", which is how the guide spells it.
        assert_eq!(
            terms_of("SFX_UI_Action mod-studio"),
            vec!["sfx_ui_action", "mod", "studio"]
        );
        assert_eq!(terms_of("game-path configuration"), vec!["game", "path", "configuration"]);
        // A word repeated in a sentence must not buy a second helping of the same evidence.
        assert_eq!(terms_of("the mod is the mod"), vec!["the", "mod", "is"]);
    }

    #[test]
    fn hyphenating_a_prose_query_does_not_change_the_answer() {
        // The defect as an agent meets it. This search takes sentences, and a sentence gets
        // hyphenated: `game-path` was one mandatory literal, so a section spelling it "game path"
        // — which is how the guide spells it — could not score at all.
        for (joined, apart) in [
            ("game-path configuration", "game path configuration"),
            ("mod-studio dialogs", "mod studio dialogs"),
        ] {
            let joined = where_(&search(joined, 5));
            let apart = where_(&search(apart, 5));
            assert!(!apart.is_empty(), "the separated query has to reach something: {apart:?}");
            assert_eq!(joined, apart, "one hyphen changed the answer");
        }
    }

    #[test]
    fn a_query_typed_as_a_sentence_keeps_the_words_at_its_end() {
        // The cap used to be eight whitespace chunks, which cut this query at `mod has` and threw
        // away `no effect` — the only two words that named the symptom.
        let terms = terms_of("deployed but nothing changed in game, mod has no effect");
        assert!(terms.contains(&"effect".to_string()), "{terms:?}");
        assert!(terms.contains(&"deployed".to_string()), "{terms:?}");
    }

    #[test]
    fn a_section_is_scored_on_its_own_prose_and_not_on_the_page_beneath_it() {
        // Sections nest: `# Textures` contains every `##` under it. Scoring that made a page title
        // unbeatable, and seven of the tester's eight hits were page titles — a section search
        // returning pages. A query lifted from one subsection's own text must reach that
        // subsection, not the title of the page it sits on.
        let page = super::super::page("textures").expect("the textures page");
        let sections = page.sections();
        let title = &sections[0];
        assert_eq!(title.level, 1, "the fixture assumes textures opens with its title");

        let hits = search("mouse cursor is not the cursor texture", 5);
        assert_eq!(
            hits[0].anchor, "the-mouse-cursor-is-not-the-cursor-texture",
            "got {:?}",
            where_(&hits)
        );
        assert_ne!(hits[0].anchor, title.anchor);
    }

    #[test]
    fn covering_more_of_the_query_beats_repeating_one_word_of_it() {
        // Rare words carry the coverage bonus, so a section that answers several parts of a symptom
        // outranks one that happens to say `deploy` a lot.
        assert!(coverage_bonus(0.0, 10.0) < coverage_bonus(5.0, 10.0));
        assert!(coverage_bonus(5.0, 10.0) < coverage_bonus(10.0, 10.0));
        // Full coverage is worth exactly the doubling the old all-or-nothing rule applied.
        assert!((coverage_bonus(10.0, 10.0) - 2.0).abs() < 1e-9);
        // A query no section can fully cover must still be ranked, not flattened.
        assert!(coverage_bonus(3.0, 10.0) > 1.0);
    }

    #[test]
    fn the_neutral_point_is_what_an_average_word_actually_leans() {
        // Why `reference_share_of_prose` and not the obvious reference-share-of-sections. Reference
        // sections are longer, so an everyday word turns up in one more often than counting
        // sections predicts: judged against the section share, a query written in plain English
        // reads as a question about internals purely because of its function words. This measures
        // the real average over every distinct word in the corpus and fails if the cheap proxy
        // used at search time drifts away from it.
        use std::collections::{HashMap, HashSet};

        let mut holding: HashMap<String, (usize, usize)> = HashMap::new();
        let mut total_sections = 0usize;
        let mut reference_sections = 0usize;

        for page in PAGES {
            let sections = page.sections();
            for index in 0..sections.len() {
                let own_end =
                    sections.get(index + 1).map_or(page.markdown.len(), |next| next.start);
                total_sections += 1;
                if page.kind == Kind::Reference {
                    reference_sections += 1;
                }
                let words: HashSet<String> = terms_of_all(&page.markdown[sections[index].start..own_end]);
                for word in words {
                    let entry = holding.entry(word).or_insert((0, 0));
                    entry.0 += 1;
                    if page.kind == Kind::Reference {
                        entry.1 += 1;
                    }
                }
            }
        }

        let measured: f64 = holding.values().map(|(all, refs)| *refs as f64 / *all as f64).sum::<f64>()
            / holding.len() as f64;
        let proxy = reference_share_of_prose();
        let by_section_count = reference_sections as f64 / total_sections as f64;

        assert!(
            (measured - proxy).abs() < 0.05,
            "the prose share {proxy:.3} no longer stands in for the average word's lean {measured:.3}"
        );
        assert!(
            measured - by_section_count > 0.05,
            "the section share {by_section_count:.3} is supposed to be the biased one, but the \
             average word leans {measured:.3}"
        );
    }

    /// Every distinct word of a passage, tokenised exactly as a query is. Only the calibration test
    /// needs this; scoring works term by term against a query.
    fn terms_of_all(text: &str) -> std::collections::HashSet<String> {
        text.to_lowercase()
            .split(|character: char| {
                !character.is_alphanumeric() && character != '_' && character != '-'
            })
            .map(|word| word.trim_matches(|character| character == '_' || character == '-'))
            .filter(|word| word.chars().count() >= 2)
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn the_guide_outranks_the_reference_unless_the_query_asks_about_internals() {
        // The tool description says the reference is for "when a command refuses something and the
        // guide does not say why". Nothing in the ranking reflected that, so receipt semantics and
        // USMAP generations came back to a beginner asking why his mod did nothing.
        let symptom = search("deployed but nothing changed in game, mod has no effect", 5);
        assert!(
            symptom.iter().take(3).all(|hit| kind_of(hit.page) == Kind::Guide),
            "a plain-English symptom must not lead with internals: {:?}",
            where_(&symptom)
        );

        // The vocabulary of the reference itself is what turns the preference off; no list of
        // words decides it.
        let internals = search("receipt seal mismatch invariant usmap generation", 5);
        assert_eq!(
            kind_of(internals[0].page),
            Kind::Reference,
            "an internals question must still reach the reference: {:?}",
            where_(&internals)
        );
    }

    fn kind_of(slug: &str) -> Kind {
        super::super::page(slug).expect("a hit names a real page").kind
    }

    #[test]
    fn the_reference_is_demoted_and_never_hidden() {
        // The demotion settles ties; it must not make a body of documentation unreachable. This
        // query is ordinary guide vocabulary, and the reference page on the same subject still
        // comes back — lower, which is the whole intent.
        let hits = search("pack the patched pair without deploying it", 25);
        assert!(
            hits.iter().any(|hit| kind_of(hit.page) == Kind::Reference),
            "no reference section survived a guide-shaped query: {:?}",
            where_(&hits)
        );
    }

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
